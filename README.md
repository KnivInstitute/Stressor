# Stressor

**Stressor** is a rust-built Windows stress tool, being made for me to learn Rust (is it better than C++? I shall seek those answers alike).

---

Have to's for next commit:
- Configure Windows VM sandbox for kernel testing
- Create and run the CPU temperature exposure kernel level driver
- Pair driver with rust application successfully
- Get kernel signed and authenticated by microsoft for not lots of $$$

## Features (to build and already built aren't differentiated until v1.0)

- **Real-Time System Monitoring**
  - Live CPU usage graph and speedometer (MHz)
  - Live RAM usage bar and history graph
  - Storage usage visualization for all drives
- **CPU Stress Test**
  - One-click stress test to push your CPU to its limits
  - Test results are logged and can be saved for later analysis
- **Test Analysis**
  - Analyze the last or any previous test in a dedicated analysis window
  - Custom format for test logs and results
- **User-Friendly**
  - Minimal setup: just run the EXE, click "Run" to start a test, and "Analyze" to review results
  - No command-line required for end users

---

## Getting Started

### Prerequisites
- **Rust** (latest stable, [install here](https://rustup.rs/))
- **Windows 10/11** (recommended)

### Running from Source

1. **Clone the repository:**
   ```sh
   git clone <repo-url>
   cd Stressor
   ```
2. **Build and run:**
   ```sh
   cargo run --release
   ```

## Usage Goal (workflow view)

1. **Launch Stressor**
   - Double-click the EXE
2. **Monitor System**
   - The main window shows live CPU, RAM, and storage stats. Here, you can verify devices that will be stressed
3. **Run a Stress Test**
   - Switch to the "Stress Test" tab and click "Run".
   - The app will stress your CPU and log the results.
   - Storage speed analyzers will also be implemented
4. **Analyze Results**
   - Use the "Analyze" feature to review the last or any saved test.
   - Results are shown in a user-friendly format.

---

## Contributing

Contributions are welcome! Please open issues or pull requests for bug fixes, features, or suggestions.

---

## License

This project is licensed under the MIT License.