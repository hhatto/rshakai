extern crate time;
use std::io::{stdout, Write};
use std::sync::mpsc::Receiver;

pub struct Indicator {
    pub rx: Receiver<Option<bool>>,
    pub success: u32,
    pub fail: u32,
    concurrency: u32,
    start_time: time::Tm,
    end_time: time::Tm,
}

impl Indicator {
    pub fn new(rx: Receiver<Option<bool>>, concurrency: u32) -> Indicator {
        Indicator {
            rx: rx,
            success: 0,
            fail: 0,
            concurrency: concurrency,
            start_time: time::now(),
            end_time: time::now(),
        }
    }

    pub fn run_forever(&mut self) {
        let mut skip = 0;
        loop {
            let success = self.rx.recv().unwrap();
            skip += 1;
            match success {
                Some(success) => {
                    if success {
                        self.success += 1;
                        if skip >= 100 {
                            print!(".");
                            stdout().flush().unwrap();
                            skip = 0;
                        }
                    } else {
                        self.fail += 1;
                        print!("x");
                        stdout().flush().unwrap();
                    }
                }
                None => {
                    break;
                }
            }
        }

        self.end_time = time::now();
        self.print_stats();
    }

    fn print_stats(&self) {
        let diff = self.end_time - self.start_time;
        let diff_msec = diff.num_milliseconds() as f32 / 1000.;
        let all_reqs = self.success + self.fail;
        println!("\nrequest count:{}, concurrency:{}, time:{:03}, {} req/s",
                 all_reqs,
                 self.concurrency,
                 diff_msec,
                 all_reqs as f32 / diff_msec);
        println!("SUCCESS {}", self.success);
        println!("FAILED {}", self.fail);
        println!("Average response time[ms]: {}", "hoge");
    }
}
