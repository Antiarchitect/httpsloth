# Httpsloth
## Intro
This tool is a proof of concept implementation of `Slow HTTP POST` denial of service attack.
The general idea behind this technique is to open thousands of cheap TCP connections with proper HTTP POST request
string containing `Content-Length` big enough and feed them with body byte by byte periodically. In case of Nginx
web server bytes should be sent in periods less than the value of `client_body_timeout` parameter which is `60s`
by default. Connections amount should exceed `worker_processes * worker_connections` values which is `1 * 1024 = 1024`
by default.

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

## Countermeasures
Proper web server configuration.
Your front-facing server should be able to hold much more open connections than the default setup allows you to.
Increase number of workers and the amount of connections each one of them limited to hold by playing with
`worker_processes` and `worker_connections` parameters in case of Nginx. Also your timeout between the consequent HTTP
body parts should be adequate and you cannot decrease its value too much in order to handle legit connection.
Recommendation here for `client_body_timeout` in case of Nginx is couple of seconds for ordinary web app. Please note
that `client_body_timeout` stands for the timeout between consequent body parts not the entire body. 