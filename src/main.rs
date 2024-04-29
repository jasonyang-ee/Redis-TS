use redis::Commands;
use redis_ts::{TsCommands, TsOptions, TsDuplicatePolicy, TsRange, TsRangeQuery, TsInfo};
use std::thread;
use std::time::{Duration, SystemTime};
use sysinfo::{System, RefreshKind, CpuRefreshKind};

fn main() {
    // Connect to the Redis server
    let client = redis::Client::open("redis://:testarossa@rpi.main.local:3015/").unwrap();
    let mut con = client.get_connection().unwrap();

	let my_opts = TsOptions::default()
	.retention_time(60000000)
	.uncompressed(false)
	.duplicate_policy(TsDuplicatePolicy::Last);

	// get all the keys
	let keys:Vec<String> = con.keys("*").unwrap();
	let initialized: bool = keys.contains(&"cpu_usage".to_string());

	if !initialized {
		let _:() = con.ts_create("cpu_usage", my_opts).unwrap();
	}

	// each loop is 10ms, loop for 1 hour
	for counts in 0..2000 {
		update_cpu_usage(&mut con);

		// print counts every 1 second
		if counts % 100 == 0 {
			let info:TsInfo = con.ts_info("cpu_usage").unwrap();
			println!("Counts:{} - Samples:{} - Memory:{}", counts, info.total_samples, info.memory_usage);
		}
	}
	
	// Print all CPU usage in the end
	print_cpu_usage(&mut con);
}

fn print_cpu_usage(con: &mut redis::Connection) {
    let data: TsRange<u64, f64> = con.ts_range(
        "cpu_usage",
        TsRangeQuery::default().from(0).to(i64::MAX)
    ).unwrap();

    // Print the data
    for entry in data.values {
        println!("{}: {}", entry.0, entry.1);
    }
}

fn update_cpu_usage(con: &mut redis::Connection) {
	// Update the system information
	let mut s = System::new_with_specifics(
		RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
	);
	
	// Wait a bit because CPU usage is based on diff.
	std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
	// Refresh CPUs again.
	s.refresh_cpu();

	// average CPU usage
	let mut cpu_usage: f32 = 0.0;
	for cpu in s.cpus() {
		cpu_usage += cpu.cpu_usage();
	}
	cpu_usage /= s.cpus().len() as f32;
	//make a i32 type for redis
	let cpu_usage_i: i32 = (cpu_usage * 100.0) as i32;

	// Get the current timestamp
	// let timestamp = SystemTime::now()
	// .duration_since(SystemTime::UNIX_EPOCH).expect("REASON")
	// .as_secs() as i64;

	// Store the CPU usage in Redis
	let _:() = con.ts_add_now("cpu_usage", cpu_usage_i).expect("REASON");

	// Sleep for 1 second
	thread::sleep(Duration::from_millis(1));
}