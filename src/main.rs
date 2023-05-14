// use itertools::Itertools;
extern crate rusqlite;

use log::{info, warn};
use rusqlite::Connection; // Result};
use std::collections::HashMap;
use std::path::Path; //, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use procfs::process::{all_processes, Process};

use dirs;

use std::process::Command;
// use procinfo::pid::cwd;

// fn type_of<T>(_: &T) -> &str {
//     return std::any::type_name::<T>();
// }

#[derive(Debug)]
struct RumPath {
    path: String,
    score: f64,
}

struct Proc {
    path: String,
}

fn main() {
    let db_file = Path::new(&dirs::cache_dir().unwrap()).join("rum.db");
    println!("db_file: {:?}", db_file);
    let conn = Connection::open(db_file).unwrap();
    let ps_table = get_ps_table();

    let _create_db = create_db(&conn);
    let _update_cwds = update_cwds(&conn, &ps_table);
    let _prune_stale_paths = prune_stale_paths(&conn);
    let _update_project_dirs = update_project_dirs(&conn);
}

fn update_cwds(conn: &Connection, ps_table: &Vec<Proc>) -> bool {
    let mut dirs = Vec::<String>::new();

    for proc in ps_table {
        dirs.push(proc.path.to_string())
    }

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

    return true;
}

fn prune_stale_paths(conn: &Connection) -> bool {
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
            println!("Found {:?} {:?} {:?}", p.path, p.score, b);
            conn.execute(
                "
                delete from paths where path = ?1 and score = ?2;
            ",
                &[&p.path, &p.score.to_string()],
            )
            .ok();
        }
    }
    return true;
}

fn update_project_dirs(conn: &Connection) -> bool {
    let command = Command::new("find")
        .args(["-L", "/home/unop/.config/", "-name", ".git", "-type", "d"])
        .current_dir("/home/unop/")
        .env("FOO", "bar")
        .output()
        .expect("failed to spawn process");

    println!("Exit status: {:?}", command.status.code());
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

        println!("{:?} {:?}", parent, push_url);
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
    return true;
}

fn create_db(conn: &Connection) -> bool {
    println!("{:?}", "Creating db");
    let create_query = "
        create table if not exists paths
            (path text primary key,
             score real,
             remote text
             );
    ";
    let foo = conn
        .execute(create_query, [])
        .expect("Unable to create database");
    println!("foo: {:?}", foo);
    return true;
}

fn get_ps_table() -> Vec<Proc> {
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
                eprint!(
                    "Error reading cwd() for pid: {:?} ({:?})\n",
                    stat.pid, stat.comm
                )
            }
        }
    }

    return res;
}
