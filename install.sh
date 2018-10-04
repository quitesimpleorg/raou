#!/bin/sh
cp target/release/raou /usr/bin/raou ; chmod o=rx /usr/bin/raou ; setcap 'cap_setuid=ep cap_setgid=ep' /usr/bin/raou
