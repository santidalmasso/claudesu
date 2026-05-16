use crate::errors::Result;
use crate::live;
use crate::printer;

pub fn run() -> Result<String> {
    let (seq, state) = live::detect_and_reconcile()?;
    Ok(printer::render_list(&seq, &state))
}
