//! FIT 文件合并库：将多个骑行/跑步 FIT 文件合并为单一会话，
//! 同时完整保留所有原始数据字段（心率、踏频、功率、海拔、距离、温度、GPS 等）。

pub mod fit_generator;
pub mod fit_parser;
pub mod fit_types;
pub mod inspector;
pub mod merger;

pub use fit_generator::{write_fit_file, FitGenerator};
pub use fit_parser::{read_fit_file, FitParser};
pub use inspector::{format_summary, summarize_file, SessionSummary};
pub use merger::{merge, merge_fit_files, FitMerger};