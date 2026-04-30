//! スリープ抑制 RAII ガード
//!
//! バッチ実行中にシステムスリープを抑制する。
//! - macOS: `/usr/bin/caffeinate -i` を子プロセスとして起動し、Drop 時に kill する
//! - Windows: `SetThreadExecutionState` で抑制し、Drop 時に解除する
//! - その他: 何もしない（`SleepGuard` は ZST）

/// スリープ抑制ガード。Drop 時に自動的にスリープを再許可する。
#[cfg(target_os = "macos")]
pub struct SleepGuard {
    caffeinate: std::process::Child,
}

#[cfg(target_os = "windows")]
pub struct SleepGuard;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub struct SleepGuard;

// ── macOS 実装 ─────────────────────────────────────────────────────────────

#[cfg(target_os = "macos")]
impl SleepGuard {
    /// caffeinate を起動してスリープを抑制する。
    ///
    /// macOS 絶対パス `/usr/bin/caffeinate` を使用（アプリバンドル起動時の PATH 不定対策）。
    pub fn new() -> Result<Self, std::io::Error> {
        let caffeinate = std::process::Command::new("/usr/bin/caffeinate")
            .arg("-i")
            .spawn()?;
        Ok(Self { caffeinate })
    }
}

#[cfg(target_os = "macos")]
impl Drop for SleepGuard {
    fn drop(&mut self) {
        if let Err(e) = self.caffeinate.kill() {
            // Drop 内でパニックしないよう warn に留める
            eprintln!("caffeinate.kill() failed: {e}");
        }
        let _ = self.caffeinate.wait();
    }
}

// ── Windows 実装 ───────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
impl SleepGuard {
    pub fn new() -> Self {
        // SAFETY: Windows API の正規の呼び出し
        unsafe {
            windows::Win32::System::Power::SetThreadExecutionState(
                windows::Win32::System::Power::ES_CONTINUOUS
                    | windows::Win32::System::Power::ES_SYSTEM_REQUIRED,
            );
        }
        Self
    }
}

#[cfg(target_os = "windows")]
impl Drop for SleepGuard {
    fn drop(&mut self) {
        unsafe {
            windows::Win32::System::Power::SetThreadExecutionState(
                windows::Win32::System::Power::ES_CONTINUOUS,
            );
        }
    }
}

// ── その他 OS 実装（no-op）────────────────────────────────────────────────

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl SleepGuard {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl Drop for SleepGuard {
    fn drop(&mut self) {}
}

// ─────────────────────────────────────────────
// テスト
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── サイクル 3-4: SleepGuard Drop の確認 ─────────────────────────────

    /// SleepGuard が正常に生成・破棄されることを確認する。
    ///
    /// macOS では実際に caffeinate を起動するが、テスト中に即断されることを想定。
    /// caffeinate 未インストール環境ではスキップされる想定はせず、
    /// `/usr/bin/caffeinate` が存在する macOS 上で CI を回す前提とする。
    #[test]
    fn sleep_guard_new_and_drop() {
        // new() が成功し、スコープアウトで Drop が呼ばれるだけで
        // パニック・hanging しないことを確認する。
        #[cfg(target_os = "macos")]
        {
            let guard = SleepGuard::new();
            assert!(guard.is_ok(), "SleepGuard::new() が失敗した: {:?}", guard.err());
            drop(guard.unwrap()); // Drop が呼ばれる
        }

        #[cfg(target_os = "windows")]
        {
            let guard = SleepGuard::new();
            drop(guard);
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let guard = SleepGuard::new();
            drop(guard);
        }
    }

    /// Drop トレイトが実装されていることをコンパイル時に確認する（型制約テスト）。
    #[test]
    fn sleep_guard_implements_drop() {
        fn assert_drop<T: Drop>() {}
        assert_drop::<SleepGuard>();
    }
}
