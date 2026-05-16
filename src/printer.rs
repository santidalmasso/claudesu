use comfy_table::{presets::UTF8_FULL, Cell, Color, ContentArrangement, Table};

use crate::live::ActiveState;
use crate::models::SequenceFile;

pub fn render_list(seq: &SequenceFile, state: &ActiveState) -> String {
    if seq.accounts.is_empty() {
        let mut out =
            String::from("no accounts stored — run `csu add` while logged in to Claude Code");
        if let ActiveState::Unstored { email } = state {
            out.push_str(&format!("\n\ncurrently logged into {email}"));
        }
        return out;
    }

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec!["#", "active", "email", "organization", "added"]);

    let mut slots: Vec<u32> = seq.accounts.keys().filter_map(|k| k.parse().ok()).collect();
    slots.sort_unstable();

    for slot in slots {
        let account = match seq.get_slot(slot) {
            Some(a) => a,
            None => continue,
        };
        let is_active = *state == ActiveState::Slot(slot);
        let marker = if is_active { "●" } else { "" };
        let org = account.organization_name.clone().unwrap_or_default();
        let added = account.added.format("%Y-%m-%d").to_string();

        let row = vec![
            Cell::new(slot),
            Cell::new(marker).fg(if is_active {
                Color::Green
            } else {
                Color::Reset
            }),
            Cell::new(&account.email),
            Cell::new(org),
            Cell::new(added),
        ];
        table.add_row(row);
    }

    let mut out = table.to_string();
    match state {
        ActiveState::Unstored { email } => {
            out.push_str(&format!(
                "\n\n⚠ logged into {email}, which isn't stored — run `csu add` to capture it",
            ));
        }
        ActiveState::LoggedOut => {
            out.push_str("\n\n⚠ no account is logged into Claude Code");
        }
        ActiveState::Slot(_) => {}
    }
    out
}

pub fn render_status(seq: &SequenceFile, state: &ActiveState) -> String {
    match state {
        ActiveState::Slot(slot) => match seq.get_slot(*slot) {
            Some(account) => format!(
                "active: slot {slot} — {} ({})",
                account.email,
                account.organization_name.as_deref().unwrap_or("no org"),
            ),
            None => format!("active slot {slot} has no matching account entry"),
        },
        ActiveState::Unstored { email } => {
            format!("logged into {email} — not stored in csu (run `csu add` to capture it)")
        }
        ActiveState::LoggedOut => "no account is logged into Claude Code".into(),
    }
}
