use std::fmt::Write;

pub const DEFAULT_EXPLORER_LISTEN: &str = "127.0.0.1:8080";
pub const DEFAULT_EXPLORER_WS_URL: &str = "ws://127.0.0.1:8545/explorer/ws";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerSummary {
    pub height: u64,
    pub epoch: u64,
    pub block_count: usize,
    pub miner_count: usize,
    pub validator_count: usize,
    pub job_count: usize,
    pub receipt_count: usize,
    pub settled_receipt_count: usize,
    pub finalized_block_count: usize,
    pub treasury_balance: u64,
    pub total_reward_balance: u64,
}

impl ExplorerSummary {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"height\":{},\"epoch\":{},\"block_count\":{},\"miner_count\":{},\"validator_count\":{},\"job_count\":{},\"receipt_count\":{},\"settled_receipt_count\":{},\"finalized_block_count\":{},\"treasury_balance\":{},\"total_reward_balance\":{}}}",
            self.height,
            self.epoch,
            self.block_count,
            self.miner_count,
            self.validator_count,
            self.job_count,
            self.receipt_count,
            self.settled_receipt_count,
            self.finalized_block_count,
            self.treasury_balance,
            self.total_reward_balance
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerBlock {
    pub height: u64,
    pub epoch: u64,
    pub hash: String,
    pub proposer: String,
    pub state_root: String,
    pub timestamp: u64,
}

impl ExplorerBlock {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"height\":{},\"epoch\":{},\"hash\":\"{}\",\"proposer\":\"{}\",\"state_root\":\"{}\",\"timestamp\":{}}}",
            self.height,
            self.epoch,
            escape_json(&self.hash),
            escape_json(&self.proposer),
            escape_json(&self.state_root),
            self.timestamp
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerAccount {
    pub address: String,
    pub is_miner: bool,
    pub is_validator: bool,
    pub balance: u64,
    pub reward_balance: u64,
    pub stake: u64,
    pub reputation: i64,
    pub settled_tensor_work: u64,
    pub pending_tensor_work: u64,
}

impl ExplorerAccount {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"address\":\"{}\",\"is_miner\":{},\"is_validator\":{},\"balance\":{},\"reward_balance\":{},\"stake\":{},\"reputation\":{},\"settled_tensor_work\":{},\"pending_tensor_work\":{}}}",
            escape_json(&self.address),
            self.is_miner,
            self.is_validator,
            self.balance,
            self.reward_balance,
            self.stake,
            self.reputation,
            self.settled_tensor_work,
            self.pending_tensor_work
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerMiner {
    pub address: String,
    pub operator_id: String,
    pub stake: u64,
    pub reputation: i64,
    pub settled_tensor_work: u64,
    pub pending_tensor_work: u64,
    pub hardware_class: String,
    pub gpu_utilization_bps: u64,
    pub reward_balance: u64,
}

impl ExplorerMiner {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"address\":\"{}\",\"operator_id\":\"{}\",\"stake\":{},\"reputation\":{},\"settled_tensor_work\":{},\"pending_tensor_work\":{},\"hardware_class\":\"{}\",\"gpu_utilization_bps\":{},\"reward_balance\":{}}}",
            escape_json(&self.address),
            escape_json(&self.operator_id),
            self.stake,
            self.reputation,
            self.settled_tensor_work,
            self.pending_tensor_work,
            escape_json(&self.hardware_class),
            self.gpu_utilization_bps,
            self.reward_balance
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerValidator {
    pub address: String,
    pub stake: u64,
    pub reputation: i64,
    pub valid_attestations: u64,
    pub missed_assignments: u64,
    pub reward_balance: u64,
}

impl ExplorerValidator {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"address\":\"{}\",\"stake\":{},\"reputation\":{},\"valid_attestations\":{},\"missed_assignments\":{},\"reward_balance\":{}}}",
            escape_json(&self.address),
            self.stake,
            self.reputation,
            self.valid_attestations,
            self.missed_assignments,
            self.reward_balance
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerReceipt {
    pub receipt_id: String,
    pub job_id: String,
    pub primitive_type: String,
    pub miner: String,
    pub tensor_work_units: u64,
    pub settled: bool,
}

impl ExplorerReceipt {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"receipt_id\":\"{}\",\"job_id\":\"{}\",\"primitive_type\":\"{}\",\"miner\":\"{}\",\"tensor_work_units\":{},\"settled\":{}}}",
            escape_json(&self.receipt_id),
            escape_json(&self.job_id),
            escape_json(&self.primitive_type),
            escape_json(&self.miner),
            self.tensor_work_units,
            self.settled
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerJob {
    pub job_id: String,
    pub primitive_type: String,
    pub deadline_block: u64,
    pub detail: String,
}

impl ExplorerJob {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"job_id\":\"{}\",\"primitive_type\":\"{}\",\"deadline_block\":{},\"detail\":\"{}\"}}",
            escape_json(&self.job_id),
            escape_json(&self.primitive_type),
            self.deadline_block,
            escape_json(&self.detail)
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplorerOverview {
    pub summary: ExplorerSummary,
    pub blocks: Vec<ExplorerBlock>,
    pub miners: Vec<ExplorerMiner>,
    pub validators: Vec<ExplorerValidator>,
    pub receipts: Vec<ExplorerReceipt>,
    pub jobs: Vec<ExplorerJob>,
}

impl ExplorerOverview {
    pub fn to_json(&self) -> String {
        format!(
            "{{\"type\":\"overview\",\"summary\":{},\"blocks\":{},\"miners\":{},\"validators\":{},\"receipts\":{},\"jobs\":{}}}",
            self.summary.to_json(),
            json_array(&self.blocks, ExplorerBlock::to_json),
            json_array(&self.miners, ExplorerMiner::to_json),
            json_array(&self.validators, ExplorerValidator::to_json),
            json_array(&self.receipts, ExplorerReceipt::to_json),
            json_array(&self.jobs, ExplorerJob::to_json)
        )
    }
}

pub fn blocks_json(blocks: &[ExplorerBlock]) -> String {
    format!(
        "{{\"type\":\"blocks\",\"blocks\":{}}}",
        json_array(blocks, ExplorerBlock::to_json)
    )
}

pub fn miners_json(miners: &[ExplorerMiner]) -> String {
    format!(
        "{{\"type\":\"miners\",\"miners\":{}}}",
        json_array(miners, ExplorerMiner::to_json)
    )
}

pub fn validators_json(validators: &[ExplorerValidator]) -> String {
    format!(
        "{{\"type\":\"validators\",\"validators\":{}}}",
        json_array(validators, ExplorerValidator::to_json)
    )
}

pub fn receipts_json(receipts: &[ExplorerReceipt]) -> String {
    format!(
        "{{\"type\":\"receipts\",\"receipts\":{}}}",
        json_array(receipts, ExplorerReceipt::to_json)
    )
}

pub fn jobs_json(jobs: &[ExplorerJob]) -> String {
    format!(
        "{{\"type\":\"jobs\",\"jobs\":{}}}",
        json_array(jobs, ExplorerJob::to_json)
    )
}

pub fn account_json(account: &ExplorerAccount) -> String {
    format!("{{\"type\":\"account\",\"account\":{}}}", account.to_json())
}

pub fn explorer_health_json(ws_url: &str) -> String {
    format!(
        "{{\"service\":\"tensorvm-explorer\",\"tensorvm_explorer_ready\":true,\"websocket_url\":\"{}\"}}",
        escape_json(ws_url)
    )
}

pub fn explorer_shell_html(ws_url: &str) -> String {
    let html_ws = escape_html(ws_url);
    let js_ws = escape_json(ws_url);
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>TensorVM Explorer</title>
<style>
:root {{ color-scheme: dark; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: #080a0f; color: #eff3f8; }}
body {{ margin: 0; background: #080a0f; }}
main {{ max-width: 1280px; margin: 0 auto; padding: 24px; }}
header {{ display: flex; align-items: end; justify-content: space-between; gap: 16px; margin-bottom: 20px; }}
h1, h2 {{ margin: 0; letter-spacing: 0; }}
h1 {{ font-size: 28px; }}
h2 {{ font-size: 16px; margin-bottom: 10px; color: #cbd5e1; }}
.status {{ font-size: 13px; color: #9ca3af; }}
.grid {{ display: grid; gap: 12px; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); margin-bottom: 18px; }}
.metric, section {{ background: #111827; border: 1px solid #263244; border-radius: 8px; }}
.metric {{ padding: 12px; }}
.label {{ color: #9ca3af; font-size: 12px; }}
.value {{ font-size: 22px; margin-top: 4px; font-variant-numeric: tabular-nums; }}
section {{ padding: 14px; margin-bottom: 14px; overflow-x: auto; }}
table {{ width: 100%; border-collapse: collapse; font-size: 13px; }}
th, td {{ text-align: left; padding: 8px 6px; border-bottom: 1px solid #263244; white-space: nowrap; }}
th {{ color: #9ca3af; font-weight: 600; }}
code {{ color: #a7f3d0; }}
.two {{ display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }}
.lookup {{ display: flex; gap: 8px; margin-bottom: 10px; }}
input {{ flex: 1; background: #0b1020; border: 1px solid #334155; border-radius: 6px; color: #eff3f8; padding: 9px 10px; }}
button {{ background: #e5e7eb; color: #111827; border: 0; border-radius: 6px; padding: 9px 12px; font-weight: 700; cursor: pointer; }}
@media (max-width: 820px) {{ main {{ padding: 16px; }} header, .two, .lookup {{ display: block; }} button {{ margin-top: 8px; width: 100%; }} }}
</style>
</head>
<body>
<main>
<header>
<div><h1>TensorVM Explorer</h1><div class="status" id="status">connecting</div></div>
<div class="status">WebSocket <code id="ws-url">{html_ws}</code></div>
</header>
<div class="grid" id="metrics"></div>
<section><h2>Account Lookup</h2><div class="lookup"><input id="account-input" placeholder="64-character address hex"><button id="account-button">Lookup</button></div><pre id="account-output"></pre></section>
<section><h2>Latest Blocks</h2><table id="blocks"></table></section>
<div class="two"><section><h2>Miners</h2><table id="miners"></table></section><section><h2>Validators</h2><table id="validators"></table></section></div>
<section><h2>Receipts</h2><table id="receipts"></table></section>
<section><h2>Jobs</h2><table id="jobs"></table></section>
</main>
<script>
const WS_URL = "{js_ws}";
const statusEl = document.getElementById("status");
function setStatus(text) {{ statusEl.textContent = text; }}
function shortHex(value) {{ return value ? value.slice(0, 12) + "..." + value.slice(-8) : ""; }}
function cell(value, code=false) {{ return code ? `<td><code>${{shortHex(String(value))}}</code></td>` : `<td>${{value}}</td>`; }}
function renderTable(id, headers, rows) {{
  const head = `<tr>${{headers.map(h => `<th>${{h}}</th>`).join("")}}</tr>`;
  document.getElementById(id).innerHTML = head + rows.join("");
}}
function metric(label, value) {{ return `<div class="metric"><div class="label">${{label}}</div><div class="value">${{value}}</div></div>`; }}
function renderOverview(data) {{
  const s = data.summary;
  document.getElementById("metrics").innerHTML = [
    metric("Height", s.height), metric("Epoch", s.epoch), metric("Blocks", s.block_count),
    metric("Miners", s.miner_count), metric("Validators", s.validator_count), metric("Receipts", s.receipt_count),
    metric("Settled", s.settled_receipt_count), metric("Jobs", s.job_count)
  ].join("");
  renderTable("blocks", ["Height", "Epoch", "Hash", "Proposer", "State Root", "Time"], data.blocks.map(b =>
    `<tr>${{cell(b.height)}}${{cell(b.epoch)}}${{cell(b.hash, true)}}${{cell(b.proposer, true)}}${{cell(b.state_root, true)}}${{cell(b.timestamp)}}</tr>`));
  renderTable("miners", ["Address", "Stake", "Settled Work", "Pending", "Hardware", "Rewards"], data.miners.map(m =>
    `<tr>${{cell(m.address, true)}}${{cell(m.stake)}}${{cell(m.settled_tensor_work)}}${{cell(m.pending_tensor_work)}}${{cell(m.hardware_class)}}${{cell(m.reward_balance)}}</tr>`));
  renderTable("validators", ["Address", "Stake", "Valid", "Missed", "Rewards"], data.validators.map(v =>
    `<tr>${{cell(v.address, true)}}${{cell(v.stake)}}${{cell(v.valid_attestations)}}${{cell(v.missed_assignments)}}${{cell(v.reward_balance)}}</tr>`));
  renderTable("receipts", ["Receipt", "Job", "Type", "Miner", "Work", "Settled"], data.receipts.map(r =>
    `<tr>${{cell(r.receipt_id, true)}}${{cell(r.job_id, true)}}${{cell(r.primitive_type)}}${{cell(r.miner, true)}}${{cell(r.tensor_work_units)}}${{cell(r.settled)}}</tr>`));
  renderTable("jobs", ["Job", "Type", "Deadline", "Detail"], data.jobs.map(j =>
    `<tr>${{cell(j.job_id, true)}}${{cell(j.primitive_type)}}${{cell(j.deadline_block)}}${{cell(j.detail)}}</tr>`));
}}
function wsRequest(payload, onData) {{
  const ws = new WebSocket(WS_URL);
  ws.onopen = () => ws.send(JSON.stringify(payload));
  ws.onmessage = event => {{ onData(JSON.parse(event.data)); ws.close(); }};
  ws.onerror = () => setStatus("websocket error");
  ws.onclose = () => {{}};
}}
function refresh() {{
  wsRequest({{type: "overview", block_limit: 12, receipt_limit: 20, job_limit: 20}}, data => {{ renderOverview(data); setStatus("live " + new Date().toLocaleTimeString()); }});
}}
document.getElementById("account-button").onclick = () => {{
  const address = document.getElementById("account-input").value.trim();
  if (!address) return;
  wsRequest({{type: "account", address}}, data => document.getElementById("account-output").textContent = JSON.stringify(data.account, null, 2));
}};
refresh();
setInterval(refresh, 3000);
</script>
</body>
</html>"#
    )
}

fn json_array<T>(items: &[T], render: fn(&T) -> String) -> String {
    let parts = items.iter().map(render).collect::<Vec<_>>();
    format!("[{}]", parts.join(","))
}

pub fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explorer_json_and_shell_include_live_websocket_contract() {
        let summary = ExplorerSummary {
            height: 2,
            epoch: 0,
            block_count: 2,
            miner_count: 10,
            validator_count: 5,
            job_count: 2,
            receipt_count: 10,
            settled_receipt_count: 10,
            finalized_block_count: 2,
            treasury_balance: 3,
            total_reward_balance: 100,
        };
        assert!(summary.to_json().contains("\"settled_receipt_count\":10"));

        let html = explorer_shell_html("ws://127.0.0.1:8545/explorer/ws?token=secret");
        assert!(html.contains("TensorVM Explorer"));
        assert!(html.contains("new WebSocket"));
        assert!(html.contains("\"overview\""));
        assert!(html.contains("ws://127.0.0.1:8545/explorer/ws?token=secret"));

        let health = explorer_health_json("ws://node/explorer/ws?token=\"local\"");
        assert!(health.contains("\\\"local\\\""));
        assert_eq!(escape_json("\"\\\n\r\t\u{7}"), "\\\"\\\\\\n\\r\\t\\u0007");
        let escaped_html = explorer_shell_html("ws://node/<explorer>&\"ws\"");
        assert!(escaped_html.contains("ws://node/&lt;explorer&gt;&amp;&quot;ws&quot;"));
    }
}
