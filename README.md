raou
====
raou is a lightweight sudo-like tool for Linux. It allows a user to 
execute programs as another user without entering the password. However,
the programs (including the parameters) a user can run are explicitly
specified by the administrator.

Originally written in C, it's now reimplemented in Rust.

By default, raou looks in  /etc/raou.d/ for config files. If you run
"raou backup", it will look for /etc/raou.d/backup.

Example config file:

user john
target_user root
path /usr/local/bin/script.sh


*user* is the name of the user who you want to give permissions to 
execute *path* as the *target_user*.  

*path* must contain the absolute path. 

Optional fields
---------------
*args*: If you want to leave out optional arguments (argv) to *path*, 
simply don't  include this. Otherwise, simply specify them
args -v -ltr 

*allow_args*: Allow arbitrary arguments, so:
raou backup /path

Will launch "path" as specified in the file for the backup entry, but 
with "/path" as argv[1] instead of the arguments specified with "args".

*no_new_privs*: Defaults to 1. Processes launched with this option active
won't be able to gain more privileges, even when they call setuid programs.

*env_vars*: A comma-separated list of environment variables to inherit
from the current environment. Everything else will be wiped (but others
like HOME, SHELL etc. will be appropriately set). 

*argv0*: Set this option if you want to provide your own value as "argv0"
The default is the name of the launched binary (not the whole path). 
