use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::{ready, Future};
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct CopyExact<'a, R: ?Sized, W: ?Sized> {
    reader: &'a mut R,
    writer: &'a mut W,
    pos: usize,
    cap: usize,
    amt: u64,
    expected: u64,
    buf: Box<[u8]>,
    readn_total: usize,
    readn_count: usize,
    writen_total: usize,
    writen_count: usize,
}

pub fn copy_exact<'a, R, W>(reader: &'a mut R, writer: &'a mut W, exact: u64) -> CopyExact<'a, R, W>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    CopyExact {
        reader,
        writer,
        amt: 0,
        expected: exact,
        pos: 0,
        cap: 0,
        buf: Box::new([0; 65536]),
        readn_total: 0,
        readn_count: 0,
        writen_total: 0,
        writen_count: 0,
    }
}

impl<R, W> Future for CopyExact<'_, R, W>
where
    R: AsyncRead + Unpin + ?Sized,
    W: AsyncWrite + Unpin + ?Sized,
{
    type Output = io::Result<(u64, usize, usize)>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<(u64, usize, usize)>> {
        loop {
            // If our buffer is empty, then we need to read some data to
            // continue.
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

                    self.readn_total += n;
                    self.readn_count += 1;
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

                    self.writen_total += i;
                    self.writen_count += 1;
                }
            }

            // If we've written al the data and we've seen EOF, flush out the
            // data and finish the transfer.
            // done with the entire transfer.
            if self.pos == self.cap && self.amt >= self.expected {
                let me = &mut *self;
                ready!(Pin::new(&mut *me.writer).poll_flush(cx))?;

                return Poll::Ready(Ok((
                    self.amt,
                    self.readn_total / self.readn_count,
                    self.writen_total / self.writen_count,
                )));
            }
        }
    }
}
