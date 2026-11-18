//! C ABI v1 over the daemon's management IPC (`ayvpn` ABI version 1).

use std::future::Future;

use crate::{Client, TunnelState};

pub const AYVPN_OK: i32 = 0;
pub const AYVPN_CHANGED: i32 = 1;
pub const AYVPN_ERR_IPC: i32 = -1;
pub const AYVPN_ERR_INPUT: i32 = -2;
pub const AYVPN_ERR_DAEMON: i32 = -3;
pub const AYVPN_ERR_RUNTIME: i32 = -4;

pub const AYVPN_STATE_DISCONNECTED: u32 = 0;
pub const AYVPN_STATE_CONNECTING: u32 = 1;
pub const AYVPN_STATE_CONNECTED: u32 = 2;
pub const AYVPN_STATE_DISCONNECTING: u32 = 3;
pub const AYVPN_STATE_ERROR: u32 = 4;

fn block_on<T>(work: impl Future<Output = Result<T, i32>>) -> Result<T, i32> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| AYVPN_ERR_RUNTIME)?
        .block_on(work)
}

async fn client() -> Result<Client, i32> {
    Client::connect().await.map_err(|_| AYVPN_ERR_IPC)
}

fn code(result: Result<bool, i32>) -> i32 {
    match result {
        Ok(true) => AYVPN_CHANGED,
        Ok(false) => AYVPN_OK,
        Err(code) => code,
    }
}

/// The `AYVPN_STATE_*` value of a tunnel state.
pub fn state_code(state: &TunnelState) -> u32 {
    match state {
        TunnelState::Disconnected { .. } => AYVPN_STATE_DISCONNECTED,
        TunnelState::Connecting { .. } => AYVPN_STATE_CONNECTING,
        TunnelState::Connected { .. } => AYVPN_STATE_CONNECTED,
        TunnelState::Disconnecting(_) => AYVPN_STATE_DISCONNECTING,
        TunnelState::Error(_) => AYVPN_STATE_ERROR,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ayvpn_abi_version() -> u32 {
    1
}

/// # Safety
/// `out_state` is null or valid for one `u32` write.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ayvpn_state(out_state: *mut u32) -> i32 {
    if out_state.is_null() {
        return AYVPN_ERR_INPUT;
    }
    match block_on(async { client().await?.state().await.map_err(|_| AYVPN_ERR_DAEMON) }) {
        Ok(state) => {
            // SAFETY: checked non-null above; the caller guarantees it is writable.
            unsafe { out_state.write(state_code(&state)) };
            AYVPN_OK
        }
        Err(code) => code,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ayvpn_connect() -> i32 {
    code(block_on(async {
        client().await?.connect_tunnel().await.map_err(|_| AYVPN_ERR_DAEMON)
    }))
}

#[unsafe(no_mangle)]
pub extern "C" fn ayvpn_disconnect() -> i32 {
    code(block_on(async {
        client().await?.disconnect_tunnel().await.map_err(|_| AYVPN_ERR_DAEMON)
    }))
}

/// # Safety
/// `account` is null or valid for `len` bytes of reads.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ayvpn_login(account: *const u8, len: usize) -> i32 {
    if account.is_null() || len > 64 {
        return AYVPN_ERR_INPUT;
    }
    // SAFETY: checked non-null and bounded above; the caller guarantees `len` readable bytes.
    let bytes = unsafe { std::slice::from_raw_parts(account, len) };
    let Ok(account) = crate::cli::read_account(bytes) else {
        return AYVPN_ERR_INPUT;
    };
    match block_on(async { client().await?.login(account).await.map_err(|_| AYVPN_ERR_DAEMON) }) {
        Ok(()) => AYVPN_OK,
        Err(code) => code,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ayvpn_daemon_run() -> i32 {
    match block_on(async { crate::run().await.map_err(|_| AYVPN_ERR_DAEMON) }) {
        Ok(()) => AYVPN_OK,
        Err(code) => code,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_input_is_refused_before_any_ipc() {
        assert_eq!(ayvpn_abi_version(), 1);
        assert_eq!(unsafe { ayvpn_state(std::ptr::null_mut()) }, AYVPN_ERR_INPUT);
        assert_eq!(unsafe { ayvpn_login(std::ptr::null(), 16) }, AYVPN_ERR_INPUT);
        let short = b"123";
        assert_eq!(unsafe { ayvpn_login(short.as_ptr(), short.len()) }, AYVPN_ERR_INPUT);
    }
}
