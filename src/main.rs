use std::env::Args;
use std::ffi::CStr;
use std::ffi::CString;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::{Error, ErrorKind};

extern crate libc;
use libc::passwd;

struct Entry {
    users: Vec<String>,
    dest_user: String,
    cmd: String,
    args: String,
    argv0: String,
    inherit_envs: Vec<String>,
    no_new_privs: bool,
    arbitrary_args: bool,
}

impl Entry {
    fn new() -> Entry {
        return Entry {
            users: Vec::new(),
            dest_user: String::new(),
            cmd: String::new(),
            args: String::new(),
            argv0: String::new(),
            inherit_envs: Vec::new(),
            no_new_privs: true,
            arbitrary_args: false,
        };
    }
}

struct Passwd {
    pw_name: String,
    pw_passwd: String,
    pw_uid: libc::uid_t,
    pw_gid: libc::gid_t,
    pw_gecos: String,
    pw_dir: String,
    pw_shell: String,
}

fn initgroups(user: &str, group: libc::gid_t) -> std::io::Result<()> {
    let userarg = CString::new(user);
    return errnowrapper(unsafe {
        libc::initgroups(userarg.unwrap().as_ptr(), group)
    });
}

fn errnowrapper(ret: libc::c_int) -> std::io::Result<()> {
    if ret != 0 {
        return Err(Error::last_os_error());
    }
    return Ok(());
}

fn setuid(id: libc::uid_t) -> std::io::Result<()> {
    return errnowrapper(unsafe { libc::setuid(id) });
}

fn setgid(gid: libc::gid_t) -> std::io::Result<()> {
    return errnowrapper(unsafe { libc::setgid(gid) });
}

fn geteuid() -> u32 {
    unsafe {
        return libc::geteuid();
    }
}

fn getpwnam(username: &str) -> std::io::Result<Passwd> {
    fn getstr(str: *mut libc::c_char) -> String {
        unsafe { CStr::from_ptr(str).to_string_lossy().into_owned() }
    }
    let username_c = CString::new(username).unwrap();
    let username_ptr = username_c.as_ptr();
    let pwnamresult: *mut libc::passwd = unsafe { libc::getpwnam(username_ptr) };
    if pwnamresult.is_null() {
        return Err(Error::new(
            Error::last_os_error().kind(),
            "Lookup of user failed: ".to_owned() +
                &Error::last_os_error().to_string(),
        ));
    }
    unsafe {
        Ok(Passwd {
            pw_name: getstr((*pwnamresult).pw_name),
            pw_passwd: getstr((*pwnamresult).pw_passwd),
            pw_uid: (*pwnamresult).pw_uid,
            pw_gid: (*pwnamresult).pw_gid,
            pw_gecos: getstr((*pwnamresult).pw_gecos),
            pw_dir: getstr((*pwnamresult).pw_dir),
            pw_shell: getstr((*pwnamresult).pw_shell),
        })
    }
}

fn ensure_allowed(userid: libc::uid_t, entry: &Entry) -> std::io::Result<()> {
    if userid == 0 {
        return Ok(());
    }
    for user in &entry.users {
        let passwd: Passwd = getpwnam(&user)?;
        if passwd.pw_uid == userid {
            return Ok(());
        }
    }

    let passwd: Passwd = getpwnam(&entry.dest_user)?;
    if passwd.pw_uid == userid {
        return Ok(());
    }
    return Err(Error::new(
        ErrorKind::PermissionDenied,
        "Not allowed to become target user",
    ));
}

fn usage() {
    println!("Usage: raou ENRTYNAME");
}
fn add_multi(vec: &mut Vec<String>, val: String) {
    if val.contains(',') {
        let splitted = val.split(',');
        for part in splitted {
            vec.push(part.to_owned());
        }
    } else {
        vec.push(val);
    }
}
fn assign(entry: &mut Entry, key: &str, value: &str) {
    let val = value.to_owned();
    match key {
        "path" => entry.cmd = val,
        "user" => add_multi(&mut entry.users, val),
        "env_vars" => add_multi(&mut entry.inherit_envs, val),
        "argv0" => entry.argv0 = val,
        "target_user" => entry.dest_user = val,
        "allow_args" => entry.arbitrary_args = val == "1" || val == "true",
        "args" => entry.args = val,
        "no_new_privs" => entry.no_new_privs = val == "1" || val == "true",
        _ => {
            eprintln!("Ignoring invalid key {}", key);
        }
    }
}
fn assign_from_line(entry: &mut Entry, line: &str) {
    let mut splitted = line.splitn(2, ' ');
    let key = splitted.next();
    let value = splitted.next();

    if !key.is_some() || !value.is_some() {
        return;
    }
    assign(entry, key.unwrap(), value.unwrap())
}
fn create_entry_from_file(filepath: &str) -> std::io::Result<Entry> {
    let mut entry: Entry = Entry::new();
    let f = File::open(filepath)?;

    let bf = BufReader::new(f);

    for line in bf.lines() {
        assign_from_line(&mut entry, &line.unwrap());
    }
    Ok(entry)
}

//TODO: clearenv does not set errno?
fn clearenv() -> std::io::Result<()> {
    return errnowrapper(unsafe { libc::clearenv() });
}
//TODO: AsRef for envs?
fn setup_environment(passwd: &Passwd, envs: &[String]) -> std::io::Result<()> {
    let saved_envs: Vec<String> = envs.iter()
        .map(|s| std::env::var(s).expect("No such var"))
        .collect();
    clearenv()?;

    //TODO: set_var does not have a return val?
    std::env::set_var("HOME", &passwd.pw_dir);
    std::env::set_var("USER", &passwd.pw_name);
    std::env::set_var("LOGNAME", &passwd.pw_name);
    std::env::set_var("SHELL", &passwd.pw_shell);

    for (i, item) in saved_envs.iter().enumerate() {
        std::env::set_var(&envs[i], item);
    }
    Ok(())
}

fn become_user(passwd: &Passwd) -> std::io::Result<()> {
    initgroups(&(passwd.pw_name), passwd.pw_gid)?;
    setgid(passwd.pw_gid)?;
    setuid(passwd.pw_uid)?;
    std::env::set_current_dir(&passwd.pw_dir)?;
    Ok(())
}

fn drop_privs(entry: &Entry) -> std::io::Result<()> {
    if entry.no_new_privs {
        errnowrapper(unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0) })?;
        errnowrapper(unsafe {
            libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0)
        })?;
    }
    Ok(())
}

#[inline(always)]
fn to_cstring<T: AsRef<str>>(s: T) -> *const libc::c_char {
    return CString::new(s.as_ref()).unwrap().into_raw();
}

fn create_execv_args(entry: &Entry, cmdargs: &Vec<String>) -> Vec<*const libc::c_char> {
    let mut args: Vec<*const libc::c_char>;
    if entry.arbitrary_args && cmdargs.len() > 2 {
        args = cmdargs.iter().skip(2).map(to_cstring).collect();
    } else {
        args = entry
            .args
            .as_str()
            .split_whitespace()
            .map(to_cstring)
            .collect();
    }
    if !&entry.argv0.is_empty() {
        args.insert(0, to_cstring(&entry.argv0));
    } else {
        let cmdbegin = &entry.cmd.rfind("/").unwrap() + 1;
        args.insert(0, to_cstring(&entry.cmd.split_at(cmdbegin).1));
    }
    args.push(std::ptr::null());
    return args;
}
fn exec(entryname: &str, cmdargs: &Vec<String>) -> std::io::Result<()> {
    let mut filepath: String = String::from("/etc/raou.d/");
    filepath = filepath + entryname;

    if !std::path::Path::new(&filepath).exists() {
        return Err(std::io::Error::new(
            ErrorKind::NotFound,
            "The entry ".to_owned() + &filepath + " does not exist",
        ));
    }
    let entry: Entry = create_entry_from_file(&filepath)?;
    let destuserpasswd: Passwd = getpwnam(&entry.dest_user)?;
    let currentuser: u32 = geteuid();

    let args = create_execv_args(&entry, &cmdargs);

    ensure_allowed(currentuser, &entry)?;
    become_user(&destuserpasswd).or_else(|e| {
        return Err(Error::new(
            ErrorKind::PermissionDenied,
            "Failed to switch user: ".to_owned() + &e.to_string(),
        ));
    })?;
    setup_environment(&destuserpasswd, &entry.inherit_envs)
        .or_else(|e| {
            return Err(Error::new(
                ErrorKind::Other,
                "Environment setup failure: ".to_owned() + &e.to_string(),
            ));
        })?;

    drop_privs(&entry).or_else(|e| {
        return Err(Error::new(
            ErrorKind::Other,
            "Failed to drop priviliges: ".to_owned() + &e.to_string(),
        ));
    })?;

    unsafe {
        errnowrapper(libc::execv(to_cstring(entry.cmd), args.as_ptr()))
            .or_else(|e| {
                return Err(Error::new(
                    ErrorKind::Other,
                    "execv failed: ".to_owned() + &e.to_string(),
                ));
            })?;
    }
    std::process::exit(0);
}
fn main() -> Result<(), std::io::Error> {
    let argv: Args = std::env::args();
    let cmdargs: Vec<String> = argv.collect();
    let entryname = cmdargs.get(1);
    if entryname.is_some() {
        match exec(&entryname.unwrap(), &cmdargs) {
            Err(e) => {
                eprintln!("The following error ocurred:");
                eprintln!("{}", e);

                std::process::exit(1);
            }
            _ => {}
        };
    }
    usage();
    std::process::exit(1);
}
