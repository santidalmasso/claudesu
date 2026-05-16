use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "csu",
    version,
    about = "Switch between multiple Claude Code accounts",
    long_about = "csu (claude su) stores Claude Code credentials in numbered slots \
                  so you can rotate between accounts without logging out. Run `csu add` while \
                  logged in to capture each account."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Capture the currently logged-in Claude Code account into a slot.
    Add {
        /// Force a specific slot number. Fails if taken.
        #[arg(long)]
        slot: Option<u32>,
    },

    /// Remove a stored account by slot number or email.
    Remove {
        /// Slot number or email of the account to remove.
        who: String,

        /// Skip the confirmation prompt.
        #[arg(long, short = 'y')]
        yes: bool,
    },

    /// Show all stored accounts.
    List,

    /// Rotate to the next slot in sequence order.
    Switch,

    /// Switch to a specific slot by number or email.
    #[command(name = "switch-to")]
    SwitchTo {
        /// Slot number or email to activate.
        who: String,
    },

    /// Show the currently active account.
    Status,

    /// Delete all csu-managed state. Active credentials are left alone.
    Purge {
        /// Skip the confirmation prompt.
        #[arg(long, short = 'y')]
        yes: bool,
    },
}
