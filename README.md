!\[first rust-endeavour\](https://badgen.net/badge/first/rust-endeavour/red?icon=rust)

`rum` is a WIP PoC to track and measure _frecently_-used files/directories.
The aim is for a small/fast tool that can integrate into editors, fzf, etc.


```
$ make build run
...

# Most frecently used git repositories

$ litecli ~/.cache/rum.db -t -e '
    select * from paths
      where score > 0.2 and
      remote is not null
    order by score desc'
+----------------------------------------------------+--------------------+-----------------------------------------------------------------+
| path                                               | score              | remote                                                          |
+----------------------------------------------------+--------------------+-----------------------------------------------------------------+
| /home/foo/sillysocks/terraform-aws-DeployerEC2     | 6.054006648674003  | https://github.com/sillysocks/terraform-aws-DeployerEC2.git     |
| /home/foo/sillysocks/project-build-tools           | 4.167641480119219  | https://un1x3@github.com/sillysocks/project-build-tools.git     |
| /home/foo/.config/dotfiles                         | 1.6375775613398182 | https://github.com/un1x3/dotfiles.git                           |
| /home/foo/sillysocks/cse-team-deploy               | 1.6375775606703813 | https://github.com/sillysocks/cse-team-deploy.git               |
| /home/foo/sillysocks/terraform-aws-DeployerS3      | 1.6262820810842502 | https://github.com/sillysocks/terraform-aws-DeployerS3.git      |
| /home/foo/sillysocks/terraform-aws-DeployerRoute53 | 1.6262820810842502 | https://github.com/sillysocks/terraform-aws-DeployerRoute53.git |
+----------------------------------------------------+--------------------+-----------------------------------------------------------------+
```

```
# Types of directories managed

/home/foo/.cache/rum.db> select
   count(score) count,
     case typeof(remote)
       when "text" then "git repository"
       when typeof(remote) then "regular directory"
     end type
   from paths
   group by typeof(remote)
   order by count(score) desc;
+-------+-------------------+
| count | type              |
+-------+-------------------+
| 313   | git repository    |
| 16    | regular directory |
+-------+-------------------+
2 rows in set
Time: 0.005s
```

#### To Do

- [x] `/proc/*/cwd`
- [x] git repository collections
- [ ] viminfo files
- [ ] open files
