//! The `ayvpn` command line: one daemon command and four client commands.

use std::io::{Read, Write};

use crate::Client;

pub const USAGE: &str = "usage: ayvpn <daemon|status|connect|disconnect|login --stdin|login --from-env NAME>";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    Daemon,
    Status,
    Connect,
    Disconnect,
    Login,
    LoginEnv(String),
}

pub fn parse(args: &[String]) -> Result<Command, &'static str> {
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["daemon"] => Ok(Command::Daemon),
        ["status"] => Ok(Command::Status),
        ["connect"] => Ok(Command::Connect),
        ["disconnect"] => Ok(Command::Disconnect),
        ["login", "--stdin"] => Ok(Command::Login),
        ["login", "--from-env", name] if valid_env_name(name) => Ok(Command::LoginEnv((*name).to_owned())),
        _ => Err(USAGE),
    }
}

fn valid_env_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

/// Reads the account from the named environment variable, never echoing its value.
pub fn account_from_env(name: &str) -> Result<String, &'static str> {
    let value = std::env::var_os(name).ok_or("the account environment variable is not set")?;
    read_account(value.as_encoded_bytes())
}

/// An account is a bounded payload of sixteen ASCII digits, never a command argument.
pub fn read_account(input: impl Read) -> Result<String, &'static str> {
    let mut bytes = Vec::with_capacity(65);
    input.take(65).read_to_end(&mut bytes).map_err(|_| "unable to read the account")?;
    let account = bytes.trim_ascii();
    if bytes.len() > 64 || account.len() != 16 || !account.iter().all(u8::is_ascii_digit) {
        return Err("the account must be exactly sixteen ASCII digits");
    }
    Ok(String::from_utf8_lossy(account).into_owned())
}

fn fail(message: impl std::fmt::Display, code: i32) -> i32 {
    let _ = writeln!(std::io::stderr(), "ayvpn: {message}"); // intentional
    code
}

/// Runs one command and returns the process exit code.
pub fn main(args: &[String]) -> i32 {
    let command = match parse(args) {
        Ok(command) => command,
        Err(usage) => return fail(usage, 2),
    };
    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => return fail(format!("async runtime: {error}"), 1),
    };
    let account = match &command {
        Command::Login => Some(read_account(std::io::stdin().lock())),
        Command::LoginEnv(name) => Some(account_from_env(name)),
        _ => None,
    };
    let account = match account {
        Some(Ok(account)) => Some(account),
        Some(Err(error)) => return fail(error, 2),
        None => None,
    };
    let result = runtime.block_on(async move {
        if command == Command::Daemon {
            return crate::run().await.map(|()| String::new());
        }
        let mut client = Client::connect().await.map_err(|error| error.to_string())?;
        let text = |error: crate::Error| error.to_string();
        match command {
            Command::Status => client.state().await.map(|state| format!("{state:?}")).map_err(text),
            Command::Connect => client
                .connect_tunnel()
                .await
                .map(|changed| format!("connect requested: {changed}"))
                .map_err(text),
            Command::Disconnect => client
                .disconnect_tunnel()
                .await
                .map(|changed| format!("disconnect requested: {changed}"))
                .map_err(text),
            // A daemon error may quote the request, so it is replaced by a fixed message.
            Command::Login | Command::LoginEnv(_) => client
                .login(account.expect("read above"))
                .await
                .map(|()| "account login completed".to_owned())
                .map_err(|_| "account login failed".to_owned()),
            Command::Daemon => unreachable!("handled above"),
        }
    });
    match result {
        Ok(reply) if reply.is_empty() => 0,
        Ok(reply) => {
            let _ = writeln!(std::io::stdout(), "{reply}"); // intentional
            0
        }
        Err(error) => fail(error, 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn login_takes_only_an_explicit_transport() {
        assert_eq!(parse(&args(&["login", "--stdin"])), Ok(Command::Login));
        assert_eq!(
            parse(&args(&["login", "--from-env", "MULLVAD_ACCOUNT"])),
            Ok(Command::LoginEnv("MULLVAD_ACCOUNT".into()))
        );
        for values in [
            vec!["login"],
            vec!["login", "1234567890123456"],
            vec!["login", "--stdin", "1234567890123456"],
            vec!["login", "--from-env", "lower"],
            vec!["login", "--from-env", "A", "B"],
            vec!["status", "--stdin"],
            vec![],
        ] {
            assert_eq!(parse(&args(&values)), Err(USAGE));
        }
    }

    #[test]
    fn account_reader_is_bounded_and_never_echoes() {
        assert_eq!(read_account(&b"1234567890123456\n"[..]).unwrap(), "1234567890123456");
        for invalid in [
            &b""[..],
            &b"123456789012345"[..],
            &b"12345678901234567"[..],
            &b"1234567890123456\n9999999999999999"[..],
            &b"123456789012345x"[..],
        ] {
            assert!(!read_account(invalid).unwrap_err().contains("12345"));
        }
        let mut cursor = std::io::Cursor::new(vec![b'1'; 1024]);
        assert!(read_account(&mut cursor).is_err());
        assert_eq!(cursor.position(), 65);
    }
}
