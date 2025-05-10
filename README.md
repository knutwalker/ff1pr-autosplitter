# ff1pr-autosplitter

An auto splitter for Final Fantasy Pixel Remaster.

Implemented:

* [ ] Start timer on new game
* [x] Splits when defeated certain enemies or gained certain items (see settings)
* [x] Split on Chaos death animation

## Usage

Activate the autosplitter from LiveSplit when creating or editing your splits for Final Fantasy.

**Alternatively**:
* Go the latest release: https://github.com/knutwalker/ff1pr-autosplitter/releases/latest
* Download the `ff1pr_autosplitter.wasm` file
* Add an 'Auto Splitting Runtime' component to you layout
* Open the settings of this configuration and point it to the downloaded file

>[!IMPORTANT]
> Use only one of these methods.
> Don't add an 'Auto Splitting Runtime' component when you have enabled the autospliiter in the splits.
> Having both splitters running will result in double splits or crashes.


## Splits

The autosplitter can split on getting certain items or having defeated certain encounters.

Check the settings either in the splits config or the layout config (depending on you installation method).

Enable the settings for splits that you are using in your route, disable everything else.

The splits should work on any route (e.g. taking Marilith before Tiamat) and any category.

Battle splits can be configured to split either as the death animation starts or as the battle fades out after going through all the spoils.


***

# Developer section

## Build from source

This auto splitter is written in Rust. In order to compile it, you need to
install the Rust compiler: [Install Rust](https://www.rust-lang.org/tools/install).

Afterwards install the WebAssembly target:
```sh
rustup target add wasm32-unknown-unknown --toolchain stable
```

The auto splitter can now be compiled:
```sh
cargo b
```

The auto splitter is then available at:
```
target/wasm32-unknown-unknown/release/ff1pr_autosplitter.wasm
```

Make sure to look into the [API documentation](https://livesplit.org/asr/asr/) for the `asr` crate.

## Development

You can use the [debugger](https://github.com/LiveSplit/asr-debugger) while
developing the auto splitter to more easily see the log messages, statistics,
dump memory, step through the code and more.

The repository comes with preconfigured Visual Studio Code tasks. During
development it is recommended to use the `Debug Auto Splitter` launch action to
run the `asr-debugger`. You need to install the `CodeLLDB` extension to run it.

You can then use the `Build Auto Splitter (Debug)` task to manually build the
auto splitter. This will automatically hot reload the auto splitter in the
`asr-debugger`.

Alternatively you can install the [`cargo
watch`](https://github.com/watchexec/cargo-watch?tab=readme-ov-file#install)
subcommand and run the `Watch Auto Splitter` task for it to automatically build
when you save your changes.

The debugger is able to step through the code. You can set breakpoints in VSCode
and it should stop there when the breakpoint is hit. Inspecting variables may
not work all the time.
