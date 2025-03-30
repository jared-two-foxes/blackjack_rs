use ratatui::{};

fn main() -> anyhow::Error<()> {
    let app = App::default();
    loop {
        ui::draw(frame, &app);
    }
    Ok(())
}