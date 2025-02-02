// Copyright (c) 2022 MASSA LABS <info@massa.net>

#[cfg(test)]
pub mod types {
    pub type Duplex = crate::tests::mock_establisher::Duplex;

    pub type Listener = crate::tests::mock_establisher::MockListener;

    pub type Connector = crate::tests::mock_establisher::MockConnector;

    pub type Establisher = crate::tests::mock_establisher::MockEstablisher;
}

#[cfg(not(test))]
/// Connection types
pub mod types {
    use massa_time::MassaTime;
    use std::{io, net::SocketAddr};
    use tokio::{
        net::{TcpListener, TcpStream},
        time::timeout,
    };
    /// duplex connection
    pub type Duplex = TcpStream;
    /// listener
    pub type Listener = DefaultListener;
    /// connector
    pub type Connector = DefaultConnector;
    /// connection establisher
    pub type Establisher = DefaultEstablisher;

    /// The listener we are using
    #[derive(Debug)]
    pub struct DefaultListener(TcpListener);

    impl DefaultListener {
        /// Accepts a new incoming connection from this listener.
        pub async fn accept(&mut self) -> io::Result<(Duplex, SocketAddr)> {
            // accept
            let (sock, mut remote_addr) = self.0.accept().await?;
            // normalize address
            remote_addr.set_ip(remote_addr.ip().to_canonical());
            Ok((sock, remote_addr))
        }
    }

    /// Initiates a connection with given timeout in milliseconds
    #[derive(Debug)]
    pub struct DefaultConnector(MassaTime);

    impl DefaultConnector {
        /// Tries to connect to address
        ///
        /// # Argument
        /// * `addr`: `SocketAddr` we are trying to connect to.
        pub async fn connect(&mut self, addr: SocketAddr) -> io::Result<Duplex> {
            match timeout(self.0.to_duration(), TcpStream::connect(addr)).await {
                Ok(Ok(sock)) => Ok(sock),
                Ok(Err(e)) => Err(e),
                Err(e) => Err(io::Error::new(io::ErrorKind::TimedOut, e)),
            }
        }
    }

    /// Establishes a connection
    #[derive(Debug)]
    pub struct DefaultEstablisher;

    impl DefaultEstablisher {
        /// Creates an Establisher.
        pub fn new() -> Self {
            DefaultEstablisher {}
        }

        /// Gets the associated listener
        ///
        /// # Argument
        /// * `addr`: `SocketAddr` we want to bind to.
        pub async fn get_listener(&mut self, addr: SocketAddr) -> io::Result<DefaultListener> {
            // Create a socket2 TCP listener to manually set the IPV6_V6ONLY flag
            // This is needed to get the same behavior on all OS
            // However, if IPv6 is disabled system-wide, you may need to bind to an IPv4 address instead.
            let domain = match addr.is_ipv4() {
                true => socket2::Domain::IPV4,
                _ => socket2::Domain::IPV6,
            };

            let socket = socket2::Socket::new(domain, socket2::Type::STREAM, None)?;

            if addr.is_ipv6() {
                socket.set_only_v6(false)?;
            }
            socket.set_nonblocking(true)?;
            socket.bind(&addr.into())?;

            // Number of connections to queue, set to the hardcoded value used by tokio
            socket.listen(1024)?;

            Ok(DefaultListener(TcpListener::from_std(socket.into())?))
        }

        /// Get the connector with associated timeout
        ///
        /// # Argument
        /// * `timeout_duration`: timeout duration in milliseconds
        pub async fn get_connector(
            &mut self,
            timeout_duration: MassaTime,
        ) -> io::Result<DefaultConnector> {
            Ok(DefaultConnector(timeout_duration))
        }
    }

    impl Default for DefaultEstablisher {
        fn default() -> Self {
            Self::new()
        }
    }
}
