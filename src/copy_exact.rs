use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::metric_names::{
    METRIC_OVERALL_DISK_WRITE_OP_SIZE, METRIC_OVERALL_NETWORK_READ_OP_SIZE,
    METRIC_OVERALL_TRANSFERRED_BYTES,
};
use futures::{ready, Future};
use metrics_runtime::Sink;
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct CopyExact<'a, R: ?Sized, W: ?Sized> {
    sink: Sink,
    reader: &'a mut R,
    writer: &'a mut W,
    pos: usize,
    cap: usize,
    amt: u64,
    expected: u64,
    buf: Box<[u8]>,
}

pub fn copy_exact<'a, R, W>(
    sink: Sink,
    reader: &'a mut R,
    writer: &'a mut W,
    exact: u64,
) -> CopyExact<'a, R, W>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    CopyExact {
        sink,
        reader,
        writer,
        amt: 0,
        expected: exact,
        pos: 0,
        cap: 0,
        buf: Box::new([0u8; 65536]),
    }
}

impl<R, W> Future for CopyExact<'_, R, W>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    type Output = io::Result<u64>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        loop {
            // if our buffer is empty, then we need to read some data to
            // continue - but making sure not to read any more than is expected
            if self.pos == self.cap && self.amt < self.expected {
                let me = &mut *self;
                let n = ready!(Pin::new(&mut *me.reader).poll_read(cx, &mut me.buf))?;
                if n == 0 {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "read 0 bytes before our number of expected bytes reached",
                    )));
                } else {
                    self.pos = 0;
                    self.cap = n;

                    self.sink
                        .record_value(METRIC_OVERALL_NETWORK_READ_OP_SIZE, n as u64);
                }
            }

            // If our buffer has some data, let's write it out!
            while self.pos < self.cap {
                let me = &mut *self;
                let i = ready!(Pin::new(&mut *me.writer).poll_write(cx, &me.buf[me.pos..me.cap]))?;
                if i == 0 {
                    return Poll::Ready(Err(io::Error::new(
                        io::ErrorKind::WriteZero,
                        "write zero byte into writer",
                    )));
                } else {
                    self.pos += i;
                    self.amt += i as u64;

                    self.sink
                        .record_value(METRIC_OVERALL_DISK_WRITE_OP_SIZE, i as u64);

                    self.sink
                        .increment_counter(METRIC_OVERALL_TRANSFERRED_BYTES, i as u64);
                }
            }

            // If we've written al the data and we've seen EOF, flush out the
            // data and finish the transfer.
            // done with the entire transfer.
            if self.pos == self.cap && self.amt >= self.expected {
                let me = &mut *self;
                ready!(Pin::new(&mut *me.writer).poll_flush(cx))?;

                return Poll::Ready(Ok(self.amt));
            }
        }
    }
}
