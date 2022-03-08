/// 日志器的实现
///
/// 由于这个操作系统还不支持动态内存，Logger 里基本存不了任何信息
struct Logger;

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        // 这里判断 Metadata，以便跳过完全不可能输出的日志解析
        // 实现内核堆分配之后应该存日志的 target
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            // 不用翻墙：https://www.wanweibaike.net/wiki-ISO/IEC_6429#%E9%A2%9C%E8%89%B2
            let color_code = match record.level() {
                log::Level::Error => "31",
                log::Level::Warn => "93",
                log::Level::Info => "34",
                log::Level::Debug => "32",
                log::Level::Trace => "90",
            };
            println!(
                "\x1b[{}m[{:>5}] {}\x1b[0m",
                color_code,
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

pub fn init(level: log::LevelFilter) {
    log::set_logger(&Logger).expect("Failed to initialize logger");
    log::set_max_level(level);
}
