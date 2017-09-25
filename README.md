# Httpsloth
## Kernel parameters.
In order to hold multiple open connections make sure you have set up high hard and soft limits for open files count.
Edit `/etc/security/limits.conf` and add:
```
*               hard    nofile          16384
*               soft    nofile          12288
```
## Run
    $ CONNECTIONS_COUNT=1200 TIMEOUT_SEC=30 URL=https://target.example.com/any/valid/post/path cargo run
