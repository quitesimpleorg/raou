raou
====
raou is a lightweight sudo-like tool for Linux. It allows a user to 
execute programs as another user without entering the password. However,
the programs (including the parameters) a user can run are explicitly
specified by the administrator.

Originally written in C, it's now reimplemented in Rust.

### When to use raou (over sudo) 
Generally, it's not a replacement for sudo. The primary use case of raou is a situation in which you would want to allow a user to run a privileged operation as root without entering passwords. You may not want to use sudo for that, particularly if you don't have it installed already. Some further arguments for raou:
   
   - Simpler config
   - Less complexity, less attack surface
   - Writte in a memory-safe language

### Config
By default, raou looks in  ```/etc/raou.d/``` for config files. If you run
"raou backup", it will look for ```/etc/raou.d/backup```.
Example config file:
```
user john
target_user root
path /usr/local/bin/script.sh
```

**user** is the name of the user who you want to give permissions to 
execute **path** as the **target_user**.  

**path** must contain the absolute path of the to be executed command. 

#### Optional fields

**args** (string): If you want to leave out optional arguments (argv) to *path*, 
simply don't  include this. Otherwise, specify them here.
```
...
args -v -ltr 
```
**allow_args** (1 or 0, default 0): Allow arbitrary arguments, so:
```
raou backup /path
```

Will execute the command specified in **path** of the  ```backup``` entry with "/path" as argv[1] instead of the argument specified with "args" in the config file.

**no_new_privs** (1 or 0, default 1): Processes launched with this option active
won't be able to gain more privileges, even when they call setuid programs. This can break some programs.

**env_vars** (string): A comma-separated list of environment variables to inherit
from the current environment. Everything else will be wiped (but others
like HOME, SHELL etc. will be appropriately set). 

**argv0** (string): Set this option if you want to provide your own value as "argv0"
The default is the name of the launched binary (not the whole path). 
