#![feature(async_closure)]
// use futures::stream::{self, StreamExt};
use futures_util::{future, pin_mut, StreamExt};

use tokio::task;
use std::time::Duration;
use tokio::time::{sleep, timeout};


#[tokio::main]
async fn main() {
    // let mut stream = stream::iter(1..=3);

    let (write, mut read) = futures_channel::mpsc::unbounded();

    let writing_future = tokio::spawn(async move {
        // write.unbounded_send("Hello from future".to_string());
        for x in 0..10 {
                sleep(Duration::from_millis(2000)).await;
            let data = x.to_string();
            write.unbounded_send(data);
        }
    });

    // writing_future.await;

    // println!("{:?}", read.next().await);
    // read.try_for_each(move |msg| {
    //     println!("{}", msg);
    //     Ok(())
    // });

    // while let Some(msg) = read.next().await {
    //     println!("{}", msg);
    // }
    if let Ok(msg) = timeout(Duration::from_secs(1), read.next()).await {
        println!("{:?}", msg);
    }
    else {
        println!("lagged out");
    }
    println!("end.");
}



/*
if let Ok(_) = timeout(Duration::from_secs(30), recv_message_from_client()).await {
    // Response was received
} else {
    // Failed to receive response
}
*/
