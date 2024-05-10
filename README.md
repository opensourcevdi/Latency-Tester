### Info
A tool for measuring the delay of remote displays.

### Building

##### Ubuntu

Install dependencies:
```bash
sudo apt install libleptonica-dev libtesseract-dev clang \
                 tesseract-ocr-eng libgtk-3-dev libclang-dev \
```
Install Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Build:
```bash
cargo build --release
```

##### Windows

Use the prebuild binaries from the [build-job](https://gitlab.rz.uni-freiburg.de/opensourcevdi/latency-tester/-/jobs).


### Usage
1. Run one instance of this program on the remote and one on your local machine.
2. Make sure both windows are visible on your primary display.
3. Enter the IP address of the machine that is reachable by both in both instances.
4. Press listen on the instance the is reachable by both and then connect on the other.
   A ping should then be displayed in both instances that is constantly updated.
5. Press start on the instance on your local machine. The timer should start running in both instances.
   Shortly afterwards, the program takes a screenshot of the primary monitor and searches for the timers of both instances and compares it. The delay should be displayed. The test can now be repeated by pressing start again.


### Known Issues
- only works on first Monitor.
- only works if both instances are not scaled (the remote desktop image must not be scaled either).
- start can be pressed multiple times.
- No Error Handling (if the program crashes run im from terminal to see what's wrong).
- Generally buggy if not used exactly as in usage :)
