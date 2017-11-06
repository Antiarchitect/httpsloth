# Httpsloth
## Intro
This tool is a proof of concept implementation of `Slow HTTP POST` denial of service attack.
The general idea is to open thousands of cheap TCP connections with proper HTTP POST request containing 
`Content-Length` big enough and feed them with body byte by byte periodically. In case of Nginx web server 
bytes should be sent in periods less than the value of `client_body_timeout` setting which is `60s` by default.
Connections amount should exceed `worker_processes * worker_connections` values which is `1 * 1024 = 1024` by default.

## Rust Setup
Official Rust setup guide can be found here: https://www.rust-lang.org/en-US/install.html

## Kernel parameters.
In order to hold multiple open connections make sure you have set up high hard and soft limits for open files count.
Edit `/etc/security/limits.conf` and add:
```
*               hard    nofile          16384
*               soft    nofile          12288
```
## Run
    $ CONNECTIONS_COUNT=1200 TIMEOUT_SEC=30 URL=https://target.example.com/any/valid/post/path cargo run
