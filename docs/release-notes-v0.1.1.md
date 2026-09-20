Still **in development** — see the v0.1.0 notes for what has and has not been
checked against hardware; nothing about that has changed.

- The Convert page's "what the target holds" notes were laid out one word per
  line; they read as sentences now.
- A hidden layer on the screen canvas keeps its faint outline but no longer
  prints its labels over the output's — in a captured state every off layer
  sits at the same rectangle and the labels piled up.
- User guide: the vendor files live beside each show
  (`shows/<id>/vendor/`), not at the library root.
- `cargo run --example save` saves a prepared show file into a library as a
  new version, the way the app's Save version does — the demo state the
  project video films is made with it.
