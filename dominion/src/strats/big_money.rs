use super::super::Player;

pub fn big_money(conn: &::Connection) {
    loop {
        match conn.recv_notification() {
            ::notify::GameOver => break,
            ::notify::YourTurn(_) => {
                conn.play_all_money();
                let resp = match conn.get_buying_power() {
                    0...2 => ::response::NoProblem, // what are you even doing with your life?
                    3...5 => conn.buy(::card::Silver),
                    6...7 => conn.buy(::card::Gold),
                    _     => conn.buy(::card::Province),
                };
                if resp.is_err() {
                    panic!("Action failed!");
                }
                conn.done();
            },
            _ => conn.not_implemented(),
        }
    }
}
