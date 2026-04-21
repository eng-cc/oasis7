use std::convert::TryFrom as _;
use std::io;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::task::{Context, Poll};

use futures::io::{AsyncRead, AsyncWrite, IoSlice, IoSliceMut};
use futures::ready;
use libp2p::core::muxing::{StreamMuxer, StreamMuxerEvent};

#[derive(Debug, Default)]
pub(crate) struct Libp2pWireByteCounters {
    inbound: AtomicU64,
    outbound: AtomicU64,
}

pub(crate) type SharedLibp2pWireByteCounters = Arc<Libp2pWireByteCounters>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Libp2pWireByteSnapshot {
    pub inbound_bytes: u64,
    pub outbound_bytes: u64,
}

pub(crate) fn init_shared_wire_byte_counters() -> SharedLibp2pWireByteCounters {
    Arc::new(Libp2pWireByteCounters::default())
}

pub(crate) fn snapshot_wire_byte_counters(
    counters: &SharedLibp2pWireByteCounters,
) -> Libp2pWireByteSnapshot {
    Libp2pWireByteSnapshot {
        inbound_bytes: counters.inbound.load(Ordering::Relaxed),
        outbound_bytes: counters.outbound.load(Ordering::Relaxed),
    }
}

pub(crate) fn record_inbound_wire_bytes(counters: &SharedLibp2pWireByteCounters, num_bytes: usize) {
    counters.inbound.fetch_add(
        u64::try_from(num_bytes).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
}

pub(crate) fn record_outbound_wire_bytes(
    counters: &SharedLibp2pWireByteCounters,
    num_bytes: usize,
) {
    counters.outbound.fetch_add(
        u64::try_from(num_bytes).unwrap_or(u64::MAX),
        Ordering::Relaxed,
    );
}

#[derive(Clone)]
#[pin_project::pin_project]
pub(crate) struct InstrumentedStreamMuxer<SMInner> {
    #[pin]
    inner: SMInner,
    counters: SharedLibp2pWireByteCounters,
}

impl<SMInner> InstrumentedStreamMuxer<SMInner> {
    pub(crate) fn new(inner: SMInner, counters: SharedLibp2pWireByteCounters) -> Self {
        Self { inner, counters }
    }
}

impl<SMInner> StreamMuxer for InstrumentedStreamMuxer<SMInner>
where
    SMInner: StreamMuxer,
{
    type Substream = InstrumentedSubstream<SMInner::Substream>;
    type Error = SMInner::Error;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<StreamMuxerEvent, Self::Error>> {
        let this = self.project();
        this.inner.poll(cx)
    }

    fn poll_inbound(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Self::Substream, Self::Error>> {
        let this = self.project();
        let inner = ready!(this.inner.poll_inbound(cx)?);
        Poll::Ready(Ok(InstrumentedSubstream {
            inner,
            counters: this.counters.clone(),
        }))
    }

    fn poll_outbound(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Self::Substream, Self::Error>> {
        let this = self.project();
        let inner = ready!(this.inner.poll_outbound(cx)?);
        Poll::Ready(Ok(InstrumentedSubstream {
            inner,
            counters: this.counters.clone(),
        }))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.project();
        this.inner.poll_close(cx)
    }
}

#[pin_project::pin_project]
pub(crate) struct InstrumentedSubstream<SMInner> {
    #[pin]
    inner: SMInner,
    counters: SharedLibp2pWireByteCounters,
}

impl<SMInner: AsyncRead> AsyncRead for InstrumentedSubstream<SMInner> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        let num_bytes = ready!(this.inner.poll_read(cx, buf))?;
        record_inbound_wire_bytes(this.counters, num_bytes);
        Poll::Ready(Ok(num_bytes))
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &mut [IoSliceMut<'_>],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        let num_bytes = ready!(this.inner.poll_read_vectored(cx, bufs))?;
        record_inbound_wire_bytes(this.counters, num_bytes);
        Poll::Ready(Ok(num_bytes))
    }
}

impl<SMInner: AsyncWrite> AsyncWrite for InstrumentedSubstream<SMInner> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        let num_bytes = ready!(this.inner.poll_write(cx, buf))?;
        record_outbound_wire_bytes(this.counters, num_bytes);
        Poll::Ready(Ok(num_bytes))
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        let this = self.project();
        let num_bytes = ready!(this.inner.poll_write_vectored(cx, bufs))?;
        record_outbound_wire_bytes(this.counters, num_bytes);
        Poll::Ready(Ok(num_bytes))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.project();
        this.inner.poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let this = self.project();
        this.inner.poll_close(cx)
    }
}
