use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::TrustModel;
use failure::{self, err_msg, Error, ResultExt};
use gpgme;
use itertools::{join, Itertools};
use std::env::{current_dir, set_current_dir};
use std::ffi::OsStr;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::process::Command;
use std::process::Stdio;

pub fn strip_ext(p: &Path) -> PathBuf {
    let mut p = p.to_owned();
    let stem = p.file_stem().expect(".gpg file extension").to_owned();
    p.set_file_name(stem);
    p
}

pub fn fingerprints_of_keys(keys: &[gpgme::Key]) -> Result<Vec<(&gpgme::Key, String)>, Error> {
    keys.iter()
        .map(|k| fingerprint_of(k).map(|fpr| (k, fpr)))
        .collect::<Result<Vec<_>, _>>()
        .with_context(|_| "Unexpectedly failed to obtain fingerprint")
        .map_err(Into::into)
}
pub fn write_at(path: &Path) -> io::Result<fs::File> {
    OpenOptions::new().create(true).write(true).truncate(true).open(path)
}

pub struct UserIdFingerprint<'a>(pub &'a gpgme::Key);
impl<'a> fmt::Display for UserIdFingerprint<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} ({})",
            join(self.0.user_ids().map(|u| u.id().unwrap_or("[none]")), ", "),
            self.0.fingerprint().unwrap_or("[no fingerprint!]")
        )
    }
}

pub struct FingerprintUserId<'a>(pub &'a gpgme::Key);
impl<'a> fmt::Display for FingerprintUserId<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} ({})",
            self.0.fingerprint().unwrap_or("[no fingerprint!]"),
            join(self.0.user_ids().map(|u| u.id().unwrap_or("[none]")), ", ")
        )
    }
}

pub fn fingerprint_of(key: &gpgme::Key) -> Result<String, failure::Error> {
    key.fingerprint()
        .map_err(|e| {
            e.map(Into::into)
                .unwrap_or_else(|| err_msg("Fingerprint extraction failed"))
        })
        .map(ToOwned::to_owned)
}

pub fn new_context() -> Result<gpgme::Context, gpgme::Error> {
    gpgme::Context::from_protocol(gpgme::Protocol::OpenPgp)
}

pub struct KeylistDisplay<'a>(pub &'a [gpgme::Key]);

impl<'a> fmt::Display for KeylistDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", join(self.0.iter().map(|k| KeyDisplay(k)), ", "))
    }
}
pub struct KeyDisplay<'a>(pub &'a gpgme::Key);

impl<'a> fmt::Display for KeyDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            join(self.0.user_ids().map(|u| u.id().unwrap_or("[none]")), ", ")
        )
    }
}

pub fn export_key_with_progress(
    ctx: &mut gpgme::Context,
    gpg_keys_dir: &Path,
    key: &gpgme::Key,
    buf: &mut Vec<u8>,
    output: &mut dyn Write,
) -> Result<(String, PathBuf), Error> {
    let (fingerprint, key_path) = export_key(ctx, gpg_keys_dir, key, buf)?;
    writeln!(
        output,
        "Exported public key for user {} to '{}'",
        KeyDisplay(key),
        key_path.display()
    )
    .ok();
    Ok((fingerprint, key_path))
}

pub fn export_key(
    ctx: &mut gpgme::Context,
    gpg_keys_dir: &Path,
    key: &gpgme::Key,
    buf: &mut Vec<u8>,
) -> Result<(String, PathBuf), Error> {
    let fingerprint = fingerprint_of(key)?;
    let key_path = gpg_keys_dir.join(&fingerprint);
    ctx.set_armor(true);
    ctx.export_keys([key].iter().cloned(), gpgme::ExportMode::empty(), &mut *buf)
        .with_context(|_| err_msg("Failed to export at least one public key with signatures."))?;
    write_at(&key_path)
        .and_then(|mut f| f.write_all(buf))
        .with_context(|_| format!("Could not write public key file at '{}'", key_path.display()))?;
    buf.clear();
    Ok((fingerprint, key_path))
}

pub fn extract_at_least_one_secret_key(
    ctx: &mut gpgme::Context,
    gpg_key_ids: &[String],
) -> Result<Vec<gpgme::Key>, Error> {
    let keys = {
        let mut keys_iter = ctx.find_secret_keys(gpg_key_ids)?;
        let keys: Vec<_> = keys_iter.by_ref().collect::<Result<_, _>>()?;

        if keys_iter.finish()?.is_truncated() {
            return Err(err_msg("The key list was truncated unexpectedly, while iterating it"));
        }
        keys
    };

    if keys.is_empty() {
        return Err(if gpg_key_ids.is_empty() {
            err_msg(
                "No existing GPG key found for which you have a secret key. \
                 Please create one with 'gpg --gen-key' and try again.",
            )
        } else {
            format_err!(
                "No secret key matched any of the given user ids: {}",
                gpg_key_ids.iter().map(|id| format!("'{}'", id)).join(", ")
            )
        });
    }

    if keys.len() > 1 && gpg_key_ids.is_empty() {
        // TODO: wrap this in a custom error and let the CLI detect the issue
        return Err(format_err!(
            "Found {} viable keys for key-ids ({}), which is ambiguous. \
             Please specify one with the --gpg-key-id argument.",
            keys.len(),
            KeylistDisplay(&keys)
        ));
    };

    Ok(keys)
}

pub struct ResetCWD {
    cwd: Result<PathBuf, io::Error>,
}
impl ResetCWD {
    pub fn from_path(next_cwd: &Path) -> Result<Self, Error> {
        let prev_cwd = current_dir();
        set_current_dir(next_cwd).with_context(|_| {
            format!(
                "Failed to temporarily change the working directory to '{}'",
                next_cwd.display()
            )
        })?;
        Ok(ResetCWD { cwd: prev_cwd })
    }
}

impl Drop for ResetCWD {
    fn drop(&mut self) {
        self.cwd.as_ref().map(set_current_dir).ok();
    }
}

pub fn run_editor(editor: &OsStr, path_to_edit: &Path) -> Result<(), Error> {
    let mut running_program = Command::new(editor)
        .arg(path_to_edit)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|_| format!("Failed to start editor program at '{}'.", editor.to_string_lossy()))?;
    let status = running_program
        .wait()
        .with_context(|_| "Failed to wait for editor to exit.")?;
    if !status.success() {
        return Err(format_err!(
            "Editor '{}' failed. Edit aborted.",
            editor.to_string_lossy()
        ));
    }
    Ok(())
}

pub fn print_causes<E, W>(e: E, mut w: W)
where
    E: Into<Error>,
    W: Write,
{
    let e = e.into();
    let causes = e.iter_chain().collect::<Vec<_>>();
    let num_causes = causes.len();
    for (index, cause) in causes.iter().enumerate() {
        if index == 0 {
            writeln!(w, "{}", cause).ok();
            if num_causes > 1 {
                writeln!(w, "Caused by: ").ok();
            }
        } else {
            writeln!(w, " {}: {}", num_causes - index, cause).ok();
        }
    }
}

pub fn flags_for_model(model: &TrustModel) -> gpgme::EncryptFlags {
    let mut flags = gpgme::EncryptFlags::empty();
    flags.set(
        gpgme::EncryptFlags::ALWAYS_TRUST,
        match *model {
            TrustModel::Always => true,
            TrustModel::GpgWebOfTrust => false,
        },
    );
    flags
}
