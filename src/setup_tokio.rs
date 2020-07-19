use crate::config::Config;
use tokio::runtime::Runtime;

/// Returns a tokio runtime configured based on our command line settings.
///
pub fn create_runtime(config: &Config) -> (Runtime, String) {
    let rt: Runtime;
    let mut rt_description: String = String::new();

    if config.asynchronous_basic {
        rt = tokio::runtime::Builder::new()
            .enable_all()
            .basic_scheduler()
            .build()
            .unwrap();
        rt_description.push_str("basic");
    } else {
        // we need our builder pre-build because we want to change settings
        // according to our command line
        let mut rt_builder = tokio::runtime::Builder::new();

        rt_builder.enable_all();
        rt_builder.threaded_scheduler();
        rt_builder.on_thread_start(|| {
            // println!("thread started");
        });
        rt_builder.on_thread_stop(|| {
            // println!("thread stopping");
        });

        rt_description.push_str("threaded ");

        if config.asynchronous_core_threads > 0 {
            rt_builder.core_threads(config.asynchronous_core_threads as usize);
            rt_description
                .push_str(format!("(cores={})/", config.asynchronous_core_threads).as_str());
        } else {
            rt_description.push_str("(cores=default)/");
        }
        if config.asynchronous_max_threads > 0 {
            rt_builder.max_threads(config.asynchronous_max_threads as usize);
            rt_description.push_str(config.asynchronous_max_threads.to_string().as_str());
            rt_description.push_str("(max)");
        } else {
            rt_description.push_str("512(max)");
        }

        rt = rt_builder.build().unwrap();
    }

    (rt, rt_description)
}
