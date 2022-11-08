# Mc Scanner

A minecraft server scanner in rust.

ex. `ip_ranges` file:
```
42.0.42.0/16
0.69.0.69/21
1.2.3.4/32
8.8.8.8/11
```

ex. `config.yaml` file:
```
num_threads:        10
rate:               500
apply_blacklist:    true
default_port:       false
port:               942
```

ex. `.env` file:
```
MONGO_DB_URI=mongodb+srv://<username>:<password>@<address>:<port>/<etc>
```

`➜  server-discover config.yaml ip_ranges`

