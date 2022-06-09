## android example

```
cargo apk build --example android_hello_world
cargo apk build -p ezlog
```

gcc hello.c -o hello -lezlog -L./target/debug
LD_LIBRARY_PATH=./target/debug ./hello