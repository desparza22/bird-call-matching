# Development

Your new bare-bones project includes minimal organization with a single `main.rs` file and a few assets.

```
project/
├─ assets/ # Any assets that are used by the app should be placed here
├─ src/
│  ├─ main.rs # main.rs is the entry point to your application and currently contains all components for the app
├─ Cargo.toml # The Cargo.toml file defines the dependencies and feature flags for your project
```

### Automatic Tailwind (Dioxus 0.7+)

As of Dioxus 0.7, there no longer is a need to manually install tailwind. Simply `dx serve` and you're good to go!

Automatic tailwind is supported by checking for a file called `tailwind.css` in your app's manifest directory (next to Cargo.toml). To customize the file, use the dioxus.toml:

```toml
[application]
tailwind_input = "my.css"
tailwind_output = "assets/out.css" # also customize the location of the out file!
```

### Tailwind Manual Install

To use tailwind plugins or manually customize tailwind, you can can install the Tailwind CLI and use it directly.

### Tailwind
1. Install npm: https://docs.npmjs.com/downloading-and-installing-node-js-and-npm
2. Install the Tailwind CSS CLI: https://tailwindcss.com/docs/installation/tailwind-cli
3. Run the following command in the root of the project to start the Tailwind CSS compiler:

```bash
npx @tailwindcss/cli -i ./input.css -o ./assets/tailwind.css --watch
```

### Serving Your App

Run the following command in the root of your project to start developing with the default platform:

```bash
dx serve
```

To run for a different platform, use the `--platform platform` flag. E.g.
```bash
dx serve --platform desktop
```

Deno also installs a command named `dx`. If `dx serve` prints npm downloads instead of building
the app, the Deno one is first on your `PATH`; run the Dioxus CLI as `~/.cargo/bin/dx` instead.

### Playing over the internet

`serve-public.sh` builds a release bundle, runs the game server on this machine
(`127.0.0.1:8090`), and opens an [ngrok](https://ngrok.com) tunnel to it:

```bash
./serve-public.sh
```

Share the `https://….ngrok-free.app` URL that ngrok prints. Press Ctrl+C to stop both the tunnel
and the game server.

- No router or firewall changes are needed. ngrok connects outward from this machine, so nothing
  has to be let in. (Port forwarding is only needed to let players reach this machine directly,
  without a tunnel.)
- The release build is much smaller than `dx serve`'s debug build (about 2.6 MB of wasm instead of
  60 MB), so it loads quickly for remote players. Building takes a few minutes; rerun the last
  build with `./serve-public.sh --skip-build`.
- On a free ngrok account the URL changes each run unless you use your account's free static
  domain: `NGROK_URL=your-name.ngrok-free.app ./serve-public.sh`. Free accounts also show visitors
  a one-time ngrok warning page before the game.
- Rooms live in the server's memory, so restarting it ends any games in progress.

