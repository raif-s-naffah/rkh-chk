# Rootkit Hunter (RKHunter) Check
Rootkit Hunter Check (`rkh-chk`) is a command line tool that helps dealing w/ warnings about _changed files_ reported after a run of [`rkhunter`](https://en.wikipedia.org/wiki/Rkhunter).

When **RKHunter** is installed and setup correctly, it's invoked daily (by `cron` and/or `anacron`). When it runs, and files it is tracking now have different hashes from known stored values, it emits warnings about them in its log, and sends an email to a designated user informing them about the situation, leaving them the job of dealing with those warnings.

A quick solution for dealing w/ those warnings would be to update **RKHunter** and all its properties, doing `rkhunter --update --propupd`. A cautious user however would usually follow this workflow:

1. Find the packages that "own" those changed files.
2. Look at recent package manager transactions to see if indeed those packages were involved.
3. When satisfied that the changes to the packages of interest were legitimate, issue individual `rkhunter --propupd` for each one of them (using the RPM's base name; i.e. w/o the _release_, _version_, _architecture_, etc... parts).

This tool helps with those tasks.


## Prerequisites
This software is distributed, for now, in source only.  To build its binary you will need the latest stable Rust.

This tool may invoke, w/ User's permission &ndash;by answering a yes/no prompt before every call&ndash; the following commands expected to be accessible from the User's `$PATH`:

   * `rkhunter` - to update itself and its properties for all or specific RPMs.
   * [`rpm`](https://en.wikipedia.org/wiki/RPM_Package_Manager) - to query which installed RPM owns a file.
   * [`dnf`](https://en.wikipedia.org/wiki/DNF_(software)) - to find history info records pertaining to an RPM.
   * [`upx`](https://upx.github.io/) (Optional) - to minimize the final size of the binary.


## Building
The `Cargo.toml` file already contains the necessary incantations to minimize the _release_ binary's size &mdash;whether you'll also be using `upx` or not.  Nevertheless, i suggest you read [this](https://github.com/johnthagen/min-sized-rust) and experiment w/ the settings to ensure they best suit your setup.

Assuming you will be using `upx`, and put the resulting executable somewhere accessible from your `$PATH` (for example `~/bin`), then when you're ready do...

```bash
$ cargo b↵
$ cargo clippy↵
$ cargo b -r↵
$ rm ~/bin/rkh-chk↵
$ upx --best --lzma -o ~/bin/rkh-chk target/release/rkh-chk↵
```

## Using
To use this tool for dealing with the last run of **RKHunter**, do `$ sudo ~/bin/rkh-chk↵`. If you need to address the penultimate one, do `$ sudo ~/bin/rkh-chk last-1↵`, etc... Remember though that _changed files_, unless and until addressed, either individually or globally, will keep causing warnings.  In other words, in practice you'll rarely need to address other than the last run.


## ChangeLog
Changes are tracked [here](CHANGELOG.md).


## License

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>. 
