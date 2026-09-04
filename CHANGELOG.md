# Version 1.0.0 (2026-09-04)

* Version 1.0.0 release.
* Use Rust 1.98.1.
* Include Version string in the output.
* Ensure all public types and functions are documented.
* Sort dependencies.
* Update dependencies to latest versions.

# Version 0.2.2 (2026-08-05)

* Limit width of printed RPM names truncating them if need be.
* Be explicit about using space as fill character when formatting.
* Update dependencies to latest versions.

# Version 0.2.1 (2026-07-25)

* Use Rust 1.97.1.
* Adjusted spacing when printing DNF frames + events.
* Corrected + improved documentation.
* Update dependencies to latest versions.

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
