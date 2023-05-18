// rust

extern crate rusqlite;

use chrono::Local;
use dirs;
use env_logger::{fmt::Color, Builder, Env};
use log::LevelFilter;
use log::{debug, info, warn};
use procfs::process::{all_processes, Process};
use rusqlite::Connection; // Result};
use std::any::type_name;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io::Write;
use std::path::Path; //, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// use procinfo::pid::cwd;

fn type_of<T>(_: &T) -> &str {
    return std::any::type_name::<T>();
}

#[derive(Debug)]
struct RumPath {
    path: String,
    score: f64,
}

struct Proc {
    path: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    Builder::new()
        .format(|buf, record| {
            let mut time_style = buf.style();
            time_style.set_bg(Color::Rgb(50, 50, 50)).set_dimmed(true);

            // let level = record.level();
            // println!("type of record.level is: {:?}", type_of(&level));
            let (fg, bg) = match record.level() {
                log::Level::Warn => (Color::Black, Color::Red),
                log::Level::Info => (Color::Black, Color::Green),
                log::Level::Debug => (Color::Black, Color::Blue),
                _ => (Color::Black, Color::Cyan),
            };
            let mut level_style = buf.style();
            level_style.set_color(fg).set_bg(bg.clone()).set_bold(false);

            let mut text_style = buf.style();
            text_style.set_color(bg).set_intense(true);

            writeln!(
                buf,
                "{} [{}]: {}",
                time_style.value(Local::now().format("%T")),
                level_style.value(record.level()),
                text_style.value(record.args())
            )
        })
        .filter(None, LevelFilter::Debug)
        .init();

    let config_file = Path::new(&dirs::config_dir().unwrap()).join("rum.yaml");
    let db_file = Path::new(&dirs::cache_dir().unwrap()).join("rum.db");

    let args: Vec<String> = env::args().collect();
    debug!("Parsed args: {:?}", &args);
    debug!("db_file: {:?}", db_file);
    debug!("config_file: {:?}", config_file);

    let conn = Connection::open(db_file).unwrap();
    let ps_table = get_ps_table();

    let _create_db = create_db(&conn);
    let _update_cwds = update_cwds(&conn, &ps_table);

    let _prune_stale_paths = prune_stale_paths(&conn);
    let _update_project_dirs = update_project_dirs(&conn);

    Ok(())
}

fn update_cwds(conn: &Connection, ps_table: &Vec<Proc>) -> Result<(), Box<dyn Error>> {
    let dirs: Vec<String> = ps_table.iter().map(|p| p.path.to_string()).collect();

    let frequencies: HashMap<&String, i8> = dirs.iter().fold(HashMap::new(), |mut map, val| {
        *map.entry(val).or_default() += 1;
        map
    });

    let secs: f64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as f64;
    let base: i32 = 2;
    let exponent = base.pow(30) as f64;
    let frecencies: HashMap<&String, f64> =
        frequencies.iter().fold(HashMap::new(), |mut map, val| {
            *map.entry(val.0).or_default() = ((*val.1 as f64) * exponent + secs) / secs;
            map
        });

    for (k, v) in frecencies {
        info!("{:.4} -> {:?} -- {:?}", v, secs, k);

        conn.execute(
            "
            insert or replace into paths (path, score)
                values (?1, ?2)
                    on conflict(path) do
                        update set score=(excluded.score + score)/2;
        ",
            &[&k, &v.to_string()],
        )
        .expect("Unable to update cwds");
    }

    Ok(())
}

fn prune_stale_paths(conn: &Connection) -> Result<(), Box<dyn Error>> {
    let mut stmt = conn
        .prepare(
            "
        SELECT path, score FROM paths
            where score is not null;
        ",
        )
        .unwrap();
    let rumpath_iter = stmt
        .query_map([], |row| {
            Ok(RumPath {
                path: row.get(0)?,
                score: row.get(1)?,
            })
        })
        .unwrap();

    for rumpath in rumpath_iter {
        let p = rumpath.unwrap();
        let b = Path::new(&p.path).exists();
        if !b {
            debug!("Pruning stale path: {:?} {:?} {:?}", p.path, p.score, b);
            conn.execute(
                "
                delete from paths where path = ?1 and score = ?2;
            ",
                &[&p.path, &p.score.to_string()],
            )
            .ok();
        }
    }

    Ok(())
}

fn update_project_dirs(conn: &Connection) -> Result<(), Box<dyn Error>> {
    debug!("Updating git project dirs");

    let command = Command::new("find")
        .args(["-L", "/home/unop/oneTakeda/", "-name", ".git", "-type", "d"])
        .current_dir("/home/unop/")
        .env("FOO", "bar")
        .output()
        .expect("failed to spawn process");

    debug!("Exit status: {:?}", command.status.code());
    let stdout = String::from_utf8(command.stdout).unwrap();
    for line in stdout.lines() {
        let parent = Path::new(line).parent().unwrap();

        let command = Command::new("git")
            .args(["remote", "get-url", "--push", "origin"])
            .current_dir(parent)
            .output()
            .expect("failed to spawn git");
        let push_url = String::from_utf8(command.stdout).unwrap();
        let score: f64 = 0.2;

        info!("Found project directory: {:?} {:?}", parent, push_url);
        conn.execute(
            "
            insert or replace into paths (path, score, remote)
                values (?1, ?2, ?3)
                    on conflict(path) do
                        update
                            set path=path,
                            score=score,
                            remote=excluded.remote;
        ",
            &[
                &parent.to_str(),
                &Some(&score.to_string()),
                &Some(&push_url),
            ],
        )
        .expect("hmm");
    }

    Ok(())
}

fn create_db(conn: &Connection) -> Result<(), Box<dyn Error>> {
    debug!("{:?}", "Creating db");
    let create_query = "
        create table if not exists paths
            (path text primary key,
             score real,
             remote text
             );
    ";
    conn.execute(create_query, [])
        .expect("Unable to create database");
    Ok(())
}

fn get_ps_table() -> Vec<Proc> {
    debug!("Examining process table");

    let mut res = Vec::<Proc>::new();

    let me = Process::myself().unwrap();
    let curpid = Process::new(me.pid).unwrap();
    let curuid = curpid.uid().unwrap();

    for prc in all_processes().unwrap() {
        if let Ok(stat) = prc.unwrap().stat() {
            let proc = Process::new(stat.pid).unwrap();
            if curuid != proc.uid().unwrap() {
                continue;
            }
            if let Ok(cwd) = proc.cwd() {
                res.push(Proc {
                    path: cwd.into_os_string().into_string().unwrap(),
                })
            } else {
                warn!(
                    "Error reading cwd() for pid: {:?} ({:?})",
                    stat.pid, stat.comm
                )
            }
        }
    }

    return res;
}
