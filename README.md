[![https://crates.io](https://img.shields.io/crates/v/sheesy-cli.svg)](https://crates.io/crates/sheesy-cli)
[![ci](https://github.com/share-secrets-safely/cli/workflows/ci/badge.svg)](https://github.com/share-secrets-safely/cli/actions?query=workflow%3Aci)

**sh**are-s**e**cr**e**ts-**s**afel**y** (_sheesy_) is a solution for managing
shared secrets in teams and build pipelines.

Like [`pass`][pass], `sy` allows to setup a vault to store secrets, and share
them with your team members and tooling.
However, it wants to be a one-stop-shop in a single binary without any dependencies except
for a `gpg` installation,
helping users to work with the `gpg` toolchain and workaround peculiarities.

[![asciicast](https://asciinema.org/a/164964.png)](https://asciinema.org/a/164964?t=14)

[pass]: https://www.passwordstore.org/

## Installation

Please read the [installation notes here][installation].

[installation]: https://share-secrets-safely.github.io/cli/installation.html

## Getting Started

The first steps showing on how to use the vault with a complete example and detailed
explanations can be found [in the book][first-steps].

[first-steps]: https://share-secrets-safely.github.io/cli/vault/first-steps.html

## Project Goals

 * **a great user experience**
   * The user experience comes first when designing the tool, making it easy for newcomers while providing experts with all the knobs to tune
   * deploy as *single binary*, without dynamically linked dependencies
 * **proven cryptography**
   * Don't reinvent the wheel, use *gpg* for crypto. It's OK to require `gpg` to be installed
     on the host
   * Thanks to *GPG* each user is identified separately through their public key
 * **automation and scripting is easy**
   * storing structured secrets is as easy as making them available in shell scripts
   * common operations like substituting secrets into a file are natively supported
   * proper program exit codes make error handling easy
 * **user management**
   * support small and large teams, as well as multiple teams, with ease
   * make use of gpg's *web of trust* to allow inheriting trust even across team boundaries, and incentivize thorough checking of keys
 * **basic access control**
   * partition your secrets and define who can access them
 * **support old wheels - pass compatibility**
   * something `pass` does really well is to setup a vault with minimal infrastructure and configuration.
     We use said infrastructure and don't reinvent the wheel.
   * This makes us **compatible with pass**, allowing you use `pass` on a `sheesy` vault with default configuration.  


## Non-Goals

 * **replicate `pass` or `gpg` functionality directly**
   * having seen what `pass` actually is and how difficult it can be to use it especially in conjunction with `gpg`, this project will not even look at the provided functionality but be driven by its project goals instead.
 * **become something like hashicorp vault**
   * this solution is strictly file based and *offline*, so it can fill be used without any additional setup.

## Why would I use `sheesy` over...

You will find various and probably biased and opinionated comparisons [in our book][compare].
However, it's a fun read, and please feel free to make PRs for corrections.

[compare]: https://share-secrets-safely.github.io/cli/compare.html

## Caveats

 * Many crypto-operations store decrypted data in a temporary file. These touch
   disk and currently might be picked up by attackers. A fix could be 'tempfile',
   which allows using a secure temporary file - however, it might make getting
   MUSL builds impossible. Static builds should still be alright.
 * GPG2 is required to use the 'sign-key' operation. The latter is required when
   trying to add new unverified recipients via `vault recipients add <fingerprint>`.

## Roadmap to Future

As you can see from the version numbers, this project dispenses major version generously.
This is mainly because, for the sake of simplicity, there is only a single version number
for the *CLI* as well as all used libraries.

Effectively, you can expect the *CLI* will change rarely, and if it does only to improve
the user experience. The more tests we write, the more certain shortcomings become
evident.

The *vault library* and its types will change much more often, but we would expect it
to settle from 5.0.

### Roadmap to 4.1

This should make the first release which can be publicised, as it should include all the
material people might need to get started using _sheesy_ comfortably.

 * [ ] Documentation for
   * [ ] vault init
   * [ ] ...
 
### Roadmap to 5.0

The GPGME dependency is also the major flaw for usability, as it eventually goes down to
the quirks of GPG itself.
[SEQUOIA](https://gitlab.com/sequoia-pgp/sequoia) is a pure-Rust implementation of the
PGP protocol, which would greatly help making *sheesy* even more usable.

  * [ ] Use SEQUOIA instead of GPGME
  * [ ] Provide a windows binary

### Roadmap to 6.0
 
#### Add the `pass` subcommand

`sy` aims to be as usable as possible, and breaks compatibility were needed to
achieve that. However, to allow people to leverage its improved portability
thanks to it being self-contained, it should be possible to let it act as a
stand-in for pass.

Even though its output won't be matched, its input will be matched perfectly, as
well as its behaviour.

  * [ ] init
   
And last but not least, there should be some sort of documentation, highlighting similarities
and differences.

 * [ ] documentation
 
#### Some usability improvements

 * [ ] Assure that the error messages provided when we can't find a partition are
    better and specific to the use case.
 * [ ] Tree mode for lists of
   * [ ] recipients
   * [ ] resources

## Development Practices

 * **test-first development**
   * protect against regression and make implementing features easy
   * user docker to test more elaborate user interactions
   * keep it practical, knowing that the Rust compiler already has your back
     for the mundane things, like unhappy code paths.
 * **safety first**
   * handle all errors, never unwrap
   * provide an error chain and make it easy to understand what went wrong.
 * **strive for an MVP and version 1.0 fast...**
   * ...even if that includes only the most common usecases.
 * **Prefer to increment major version rapidly...**
   * ...instead of keeping major version zero for longer than needed.

## Maintenance Guide

### Making a release

As a prerequisite, you should be sure the build is green.

 * run `clippy` and fix all warnings with `cargo clippy --all-features --bin=sy`
 * change the version in the `VERSION` file
 * update the release notes in the `release.md` file.
   * Just prefix it with a description of new features and fixes
 * run `make tag-release`
   * requires push permissions to this repository
   * requires maintainer or owner privileges on crates.io for all deployed crates

### Making a deployment

As a prerequisite you must have made a release and your worktree must be clean,
with the HEAD at a commit.

For safety, tests will run once more as CI doesn't prevent you from publishing
red builds just yet.

  * run `make deployment`.
  * copy all text from the `release.md` file and copy it into the release text on github.
  * drag & drop all _tar.gz_  into the release and publish it.
  * in `doc/src/installation.md`, update the URL to use the latest published version
  * run `make update-homebrew` - it will push for you
  * run `make update-getting-started` - it will push for you

### Making a new Asciinema recording

Even though the documentation is currently updated with every push to master (to allows
fixing the existing docs easily), the *eye-candy* on the front page needs to be regenerated
too.

As a prerequisite, you will need an installed binary of [`asciinema`][asciinema].
Please make sure your player is already linked to your account via `asciinema auth`.

 * Set your terminal to a size of 120x20
   * You see these units when resizing an iterm2/3 terminal window
 * run `make asciinema-no-upload` and verify it contains what you expect with
   `asciicast play getting-started.cast`
 * Possibly upload the recording with `make asciinema-upload`
   * Enter the given URL and configure the asciicast to your liking, add backlinks
     to the description, and make it nice.

[asciinema]: https://asciinema.org
