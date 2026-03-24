use std::{
    env, fs,
    io::ErrorKind,
    path::Path,
    sync::{
        Arc,
        atomic::{self, AtomicBool},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use anyhow::Result;
use byteorder::ReadBytesExt;
use crossbeam::channel::{Receiver, Sender, bounded};
use log::{debug, error, info, trace};
use mio::{Events, Interest, Poll, Token, net::UnixListener};
use retry::{OperationResult, delay::Fixed, retry};

pub enum Command {
    Idle,
    Active,
    Increase(i8),
    Decrease(i8),
}

pub struct ControlServer {
    poll: Poll,
    listener: UnixListener,
    command_sender: Sender<Command>,
}

impl ControlServer {
    pub fn new() -> Result<(Self, Receiver<Command>)> {
        let socket_path = Path::new(&env::temp_dir()).join("ambient_brightness.sock");
        match fs::remove_file(socket_path.clone()) {
            Ok(()) => (),
            Err(e) if e.kind() == ErrorKind::NotFound => (),
            err => err?,
        };
        let mut listener = UnixListener::bind(socket_path)?;
        let poll = Poll::new()?;
        poll.registry().register(
            &mut listener,
            Token(0),
            Interest::READABLE | Interest::WRITABLE,
        )?;
        let (command_sender, command_receiver) = bounded(1);

        Ok((
            Self {
                poll,
                listener,
                command_sender,
            },
            command_receiver,
        ))
    }

    pub fn run(mut self, exit_bool: Arc<AtomicBool>) -> JoinHandle<Result<()>> {
        thread::spawn(move || {
            let mut events = Events::with_capacity(1024);

            loop {
                if exit_bool.load(atomic::Ordering::Relaxed) {
                    info!("Control Server Shutting Down");
                    break;
                }

                retry(Fixed::from_millis(100), || {
                    match self
                        .poll
                        .poll(&mut events, Some(Duration::from_millis(100)))
                    {
                        Ok(_) => OperationResult::Ok(()),
                        Err(e) => match e.kind() {
                            ErrorKind::Interrupted => OperationResult::Retry(e),
                            _ => {
                                error!("Poll Error: {:?}", e);
                                OperationResult::Err(e)
                            }
                        },
                    }
                })?;

                for event in &events {
                    trace!("Event: {:?}", event);

                    if event.token() == Token(0) && event.is_readable() {
                        let (mut socket, _addr) = retry(Fixed::from_millis(100).take(3), || {
                            match self.listener.accept() {
                                Err(e) => match e.kind() {
                                    ErrorKind::Interrupted => OperationResult::Retry(e),
                                    _ => {
                                        error!("Accept Error: {:?}", e);
                                        OperationResult::Err(e)
                                    }
                                },
                                Ok(socket_addr) => OperationResult::Ok(socket_addr),
                            }
                        })?;

                        let socket_read =
                            retry(Fixed::from_millis(100).take(3), || match socket.read_u8() {
                                Err(e) => match e.kind() {
                                    ErrorKind::Interrupted => OperationResult::Retry(e),
                                    _ => {
                                        error!("Read Error: {:?}", e);
                                        OperationResult::Err(e)
                                    }
                                },
                                Ok(socket_read) => OperationResult::Ok(socket_read),
                            })?;

                        debug!("Got Message: {}", socket_read);

                        match socket_read {
                            0 => self.command_sender.send(Command::Idle)?,
                            1 => self.command_sender.send(Command::Active)?,
                            2 => {
                                let amount = socket.read_i8()?;
                                self.command_sender.send(Command::Increase(amount))?
                            }
                            3 => {
                                let amount = socket.read_i8()?;
                                self.command_sender.send(Command::Decrease(amount))?
                            }
                            _ => (),
                        }
                    }
                }
            }

            Ok(())
        })
    }
}
