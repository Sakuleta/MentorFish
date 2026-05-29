// ─── Stockfish UCI Manager ───
//
// Spawns Stockfish as a child process, communicates via UCI protocol
// over async stdin/stdout channels. Implements the EngineManager trait.

use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};
use tokio::sync::Mutex;
use tokio::time::timeout;

use super::{CandidateLine, EngineConfig, EngineManager, EngineOutput};
use crate::{UCIMove, FEN};

pub struct StockfishManager {
    config: EngineConfig,
    inner: Mutex<Option<EngineProcess>>,
}

struct EngineProcess {
    _child: Child,
    stdin: ChildStdin,
    stdout_lines: tokio::sync::mpsc::Receiver<String>,
}

impl StockfishManager {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            inner: Mutex::new(None),
        }
    }

    async fn ensure_started(&self) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        self.ensure_started_inner(&mut guard).await
    }

    /// Spawn Stockfish if not already running. Takes a pre-acquired lock guard.
    async fn ensure_started_inner(&self, guard: &mut Option<EngineProcess>) -> anyhow::Result<()> {
        if let Some(ref mut proc) = *guard {
            if proc.is_alive().await {
                return Ok(());
            }
            // Process died, clean up
            *guard = None;
        }

        let mut child = tokio::process::Command::new(&self.config.binary_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child.stdin.take().expect("Failed to capture stdin");
        let stdout = child.stdout.take().expect("Failed to capture stdout");

        let (tx, rx) = tokio::sync::mpsc::channel::<String>(256);
        let reader = BufReader::new(stdout);
        tokio::spawn(async move {
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if tx.send(line).await.is_err() {
                    break;
                }
            }
        });

        let mut process = EngineProcess {
            _child: child,
            stdin,
            stdout_lines: rx,
        };

        // ─── UCI Initialization Sequence ───
        process.send_command("uci").await?;
        loop {
            let line = process.read_line().await?;
            if line == "uciok" {
                break;
            }
            if line.starts_with("option name ") {
                // UCI options are parsed but not currently used
            }
        }

        let config_lines = vec![
            format!("setoption name Threads value {}", self.config.threads),
            format!("setoption name Hash value {}", self.config.hash_mb),
            format!("setoption name MultiPV value {}", self.config.multipv),
        ];
        for cmd in config_lines {
            process.send_command(&cmd).await?;
        }
        if let Some(ref path) = self.config.syzygy_path {
            process
                .send_command(&format!("setoption name SyzygyPath value {}", path))
                .await?;
        }

        process.send_command("isready").await?;
        loop {
            let line = process.read_line().await?;
            if line == "readyok" {
                break;
            }
        }

        *guard = Some(process);
        Ok(())
    }

    async fn restart(&self) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        if let Some(mut proc) = guard.take() {
            let _ = proc.send_command("quit").await;
            let _ = proc._child.kill().await;
            // Wait for the process to actually exit to prevent zombies
            let _ = proc._child.wait().await;
        }
        // Keep lock held while spawning new process to prevent race conditions
        self.ensure_started_inner(&mut guard).await
    }

    async fn try_analyze(
        &self,
        fen: &FEN,
        depth: Option<u32>,
        on_progress: Option<super::EngineProgressFn>,
    ) -> anyhow::Result<EngineOutput> {
        let mut guard = self.inner.lock().await;
        let proc = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Engine not started"))?;

        let search_depth = depth.unwrap_or(self.config.depth);

        // Validate FEN: reject strings containing newlines or semicolons to prevent UCI injection
        if fen.contains('\n') || fen.contains('\r') || fen.contains(';') {
            anyhow::bail!("Invalid FEN: contains prohibited characters (newline/semicolon)");
        }
        log::debug!(
            "Stockfish: sending 'position fen {}' + 'go depth {}'",
            fen,
            search_depth
        );
        proc.send_command(&format!("position fen {}", fen)).await?;
        proc.send_command(&format!("go depth {}", search_depth))
            .await?;

        let mut acc = AnalysisAccumulator {
            fen: fen.clone(),
            ..Default::default()
        };
        let mut line_count = 0u32;

        loop {
            let line = proc.read_line().await?;
            line_count += 1;

            if line.starts_with("info") {
                acc.feed_info(&line);
                if let Some(ref cb) = on_progress {
                    let depth: u32 = parse_info_key(&line, "depth")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(acc.depth);
                    let eval_cp: i32 = parse_info_key(&line, "cp")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(acc.eval_cp.unwrap_or(0));
                    let nodes: Option<u64> =
                        parse_info_key(&line, "nodes").and_then(|v| v.parse().ok());
                    cb(depth, eval_cp, nodes);
                }
            } else if line.starts_with("bestmove") {
                log::debug!(
                    "Stockfish: got bestmove after {} lines: {}",
                    line_count,
                    line
                );
                acc.feed_bestmove(&line);
                break;
            } else if !line.is_empty() {
                log::debug!("Stockfish: unexpected line: {}", line);
            }
        }

        Ok(acc.into_output())
    }
}

impl EngineProcess {
    async fn send_command(&mut self, cmd: &str) -> anyhow::Result<()> {
        self.stdin
            .write_all(format!("{}\n", cmd).as_bytes())
            .await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn read_line(&mut self) -> anyhow::Result<String> {
        self.stdout_lines
            .recv()
            .await
            .ok_or_else(|| anyhow::anyhow!("Stockfish stdout stream closed"))
    }

    async fn is_alive(&mut self) -> bool {
        match self._child.try_wait() {
            Ok(Some(_)) => false, // exited
            Ok(None) => true,     // still running
            Err(_) => false,
        }
    }
}

// ─── UCI Output Parsing ───

#[derive(Debug, Default)]
struct AnalysisAccumulator {
    eval_cp: Option<i32>,
    eval_mate: Option<i32>,
    best_move: Option<UCIMove>,
    best_move_san: Option<String>,
    ponder: Option<UCIMove>,
    depth: u32,
    nodes: Option<u64>,
    nps: Option<u64>,
    time_ms: Option<u64>,
    multipv_lines: Vec<CandidateLine>,
    fen: String,
}

impl AnalysisAccumulator {
    fn feed_info(&mut self, line: &str) {
        let parts = parse_info_line(line);

        if let Some(d) = parts.get("depth") {
            self.depth = d.parse().unwrap_or(self.depth);
        }
        if let Some(n) = parts.get("nodes") {
            self.nodes = Some(n.parse().unwrap_or(0));
        }
        if let Some(n) = parts.get("nps") {
            self.nps = Some(n.parse().unwrap_or(0));
        }
        if let Some(t) = parts.get("time") {
            self.time_ms = Some(t.parse().unwrap_or(0));
        }

        let multipv: u32 = parts
            .get("multipv")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let eval_cp = parts.get("cp").and_then(|v| v.parse().ok());
        let eval_mate = parts.get("mate").and_then(|v| v.parse().ok());

        let pv: Vec<UCIMove> = parts
            .get("pv")
            .map(|v| v.split_whitespace().map(String::from).collect())
            .unwrap_or_default();

        if multipv == 1 || self.multipv_lines.is_empty() {
            self.eval_cp = eval_cp;
            self.eval_mate = eval_mate;
        }

        if let Some(existing) = self.multipv_lines.iter_mut().find(|l| l.multipv == multipv) {
            if self.depth >= existing.depth {
                existing.pv = pv;
                existing.eval_cp = eval_cp;
                existing.eval_mate = eval_mate;
                existing.depth = self.depth;
            }
        } else {
            self.multipv_lines.push(CandidateLine {
                multipv,
                pv,
                eval_cp,
                eval_mate,
                depth: self.depth,
            });
        }
    }

    fn feed_bestmove(&mut self, line: &str) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() >= 2 {
            self.best_move = Some(tokens[1].to_string());
        }
        if tokens.len() >= 4 && tokens[2] == "ponder" {
            self.ponder = Some(tokens[3].to_string());
        }
    }

    fn into_output(self) -> EngineOutput {
        EngineOutput {
            fen: self.fen,
            eval_cp: self.eval_cp.unwrap_or(0),
            eval_mate: self.eval_mate,
            best_move: self.best_move,
            best_move_san: self.best_move_san,
            ponder: self.ponder,
            depth: self.depth,
            multipv: self.multipv_lines,
            nodes: self.nodes,
            nps: self.nps,
            time_ms: self.time_ms,
        }
    }
}

/// Extract a single value for a given key from a UCI `info` line.
fn parse_info_key<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let mut i = 1; // skip "info"
    while i < tokens.len() {
        let k = tokens[i];
        if k == "score" && i + 1 < tokens.len() {
            let score_type = tokens[i + 1];
            if score_type == key && i + 2 < tokens.len() {
                return Some(tokens[i + 2]);
            }
            i += 3;
            continue;
        }
        if k == key && i + 1 < tokens.len() {
            return Some(tokens[i + 1]);
        }
        if k == "pv" {
            break;
        }
        i += 2;
    }
    None
}

/// Parse a UCI `info` line into owned key-value pairs.
fn parse_info_line(line: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let mut i = 1; // skip "info"

    while i < tokens.len() {
        let key = tokens[i];

        // "score cp" / "score mate" — compound key
        if key == "score" && i + 1 < tokens.len() {
            let score_type = tokens[i + 1];
            if let Some(value) = tokens.get(i + 2) {
                map.insert(score_type.to_string(), value.to_string());
            }
            i += 3;
            continue;
        }

        // "pv" — collect all remaining tokens as space-separated
        if key == "pv" {
            let pv_str = tokens[i + 1..].join(" ");
            map.insert("pv".to_string(), pv_str);
            break;
        }

        if i + 1 < tokens.len() {
            map.insert(key.to_string(), tokens[i + 1].to_string());
            i += 2;
        } else {
            i += 1;
        }
    }

    map
}

// ─── EngineManager Implementation ───

#[async_trait]
impl EngineManager for StockfishManager {
    async fn configure(&self, _config: EngineConfig) -> anyhow::Result<()> {
        anyhow::bail!("Reconfiguration not supported: create a new StockfishManager");
    }

    async fn analyze(
        &self,
        fen: &FEN,
        depth: Option<u32>,
        on_progress: Option<super::EngineProgressFn>,
    ) -> anyhow::Result<EngineOutput> {
        self.ensure_started().await?;

        // Prevent indefinite hangs — cap any single analysis at 30 s.
        let result = timeout(
            Duration::from_secs(30),
            self.try_analyze(fen, depth, on_progress.clone()),
        )
        .await;

        match result {
            Ok(Ok(output)) => Ok(output),
            Ok(Err(_)) => {
                log::warn!("Engine analysis failed, restarting Stockfish...");
                self.restart().await?;
                self.try_analyze(fen, depth, on_progress).await
            }
            Err(_) => {
                log::warn!("Engine analysis timed out (30 s), restarting Stockfish...");
                self.restart().await?;
                self.try_analyze(fen, depth, on_progress).await
            }
        }
    }

    async fn best_move(&self, fen: &FEN, depth: Option<u32>) -> anyhow::Result<UCIMove> {
        let output = self.analyze(fen, depth, None).await?;
        output
            .best_move
            .ok_or_else(|| anyhow::anyhow!("Stockfish did not return a best move"))
    }

    async fn configure_strength(&self, elo: Option<u32>) -> anyhow::Result<()> {
        self.ensure_started().await?;
        let mut guard = self.inner.lock().await;
        let proc = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Engine not started"))?;

        if let Some(elo_val) = elo {
            proc.send_command("setoption name UCI_LimitStrength value true")
                .await?;
            proc.send_command(&format!("setoption name UCI_Elo value {}", elo_val))
                .await?;
        } else {
            proc.send_command("setoption name UCI_LimitStrength value false")
                .await?;
        }
        Ok(())
    }

    async fn set_uci_option(&self, name: &str, value: &str) -> anyhow::Result<()> {
        self.ensure_started().await?;
        let mut guard = self.inner.lock().await;
        let proc = guard
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Engine not started"))?;
        proc.send_command(&format!("setoption name {} value {}", name, value))
            .await?;
        Ok(())
    }

    async fn health_check(&self) -> anyhow::Result<bool> {
        // Ensure the engine is spawned before checking health.
        // This is a deliberate side-effect: the first health check starts Stockfish
        // so the status dot goes green without requiring a user-triggered analysis.
        let _ = self.ensure_started().await;

        let mut guard = self.inner.lock().await;
        if let Some(ref mut proc) = guard.as_mut() {
            if proc.is_alive().await {
                return Ok(true);
            }
            // Process is dead — clean up.
            *guard = None;
        }
        Ok(false)
    }

    async fn shutdown(&self) -> anyhow::Result<()> {
        let mut guard = self.inner.lock().await;
        if let Some(mut proc) = guard.take() {
            proc.send_command("quit").await.ok();
        }
        Ok(())
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_info_line_basic() {
        let line = "info depth 10 seldepth 15 multipv 1 score cp 25 nodes 1000 nps 50000 hashfull 5 tbhits 0 time 20 pv e2e4 e7e5";
        let parts = parse_info_line(line);
        assert_eq!(parts.get("depth"), Some(&"10".to_string()));
        assert_eq!(parts.get("cp"), Some(&"25".to_string()));
        assert_eq!(parts.get("nodes"), Some(&"1000".to_string()));
        assert_eq!(parts.get("multipv"), Some(&"1".to_string()));
        assert_eq!(parts.get("pv"), Some(&"e2e4 e7e5".to_string()));
    }

    #[test]
    fn test_parse_info_line_mate() {
        let line = "info depth 5 seldepth 5 multipv 1 score mate 3 nodes 100 time 10 pv d8h4";
        let parts = parse_info_line(line);
        assert_eq!(parts.get("mate"), Some(&"3".to_string()));
    }

    #[test]
    fn test_analysis_accumulator() {
        let mut acc = AnalysisAccumulator::default();
        acc.fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string();

        acc.feed_info(
            "info depth 10 seldepth 13 multipv 1 score cp 25 nodes 1000 nps 50000 time 20 pv e2e4 e7e5 g1f3",
        );
        acc.feed_bestmove("bestmove e2e4 ponder e7e5");

        let output = acc.into_output();
        assert_eq!(output.eval_cp, 25);
        assert_eq!(output.best_move, Some("e2e4".to_string()));
        assert_eq!(output.ponder, Some("e7e5".to_string()));
        assert_eq!(output.depth, 10);
    }

    #[test]
    fn test_analysis_accumulator_multipv() {
        let mut acc = AnalysisAccumulator::default();
        acc.fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string();

        acc.feed_info("info depth 5 multipv 1 score cp 30 pv e2e4");
        acc.feed_info("info depth 5 multipv 2 score cp 20 pv d2d4");
        acc.feed_bestmove("bestmove e2e4");

        let output = acc.into_output();
        assert_eq!(output.eval_cp, 30);
        assert_eq!(output.multipv.len(), 2);
    }
}
