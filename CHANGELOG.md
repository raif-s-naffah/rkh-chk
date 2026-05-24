# Version 0.2.0 (2026-05-24)

* Collect + list unclaimed changed files -> increase minor version.
* Visually distinguish between information messages, prompts and errors when
  printing directly to the console.
* Never print style characters when logging.
* Corrected + improved documentation.
* Load configuration using path, not filename.
* Placing `.env` in `/etc` causes RKHunter to complain. Rather than requiring
  updating RKHunter's database after every change to `.env` contents, we now
  expect this file to be in the same directory where the binary is invoked from.
* Use latest `Cargo.lock`.

# Version 0.1.0 (2026-05-17)

* Initial push to GitHub.
