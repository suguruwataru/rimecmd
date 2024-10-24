use crate::json_request_processor::Request;
use crate::{Error, Result};
use mio::{unix::SourceFd, Events, Interest, Poll, Token};
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::fd::AsRawFd;
use std::rc::Rc;

pub trait RequestSource {
    fn next_request(&mut self) -> Result<Request>;
    fn register(&self, poll_request: &mut PollRequest) -> Result<()>;
}

pub struct PollRequest {
    poll: Poll,
    counter: usize,
    token_source_map: HashMap<usize, Rc<RefCell<dyn RequestSource>>>,
    result_buffer: Vec<Request>,
}

impl PollRequest {
    pub fn new(sources: &[Rc<RefCell<dyn RequestSource>>]) -> Result<Self> {
        let mut poll_request = Self {
            poll: Poll::new()?,
            counter: 0,
            result_buffer: vec![],
            token_source_map: HashMap::new(),
        };
        for source in sources.into_iter() {
            poll_request
                .token_source_map
                .insert(poll_request.counter, Rc::clone(source));
            source.borrow().register(&mut poll_request)?;
        }
        Ok(poll_request)
    }

    pub fn register(&mut self, source: &impl AsRawFd) -> Result<()> {
        self.poll.registry().register(
            &mut SourceFd(&source.as_raw_fd()),
            Token(self.counter),
            Interest::READABLE,
        )?;
        self.counter += 1;
        Ok(())
    }

    pub fn poll(&mut self) -> Result<Request> {
        let mut ret = self.result_buffer.pop();
        if ret.is_some() {
            return Ok(ret.unwrap());
        }
        let mut events = Events::with_capacity(self.counter);
        self.poll.poll(&mut events, None)?;
        for event in events.into_iter() {
            if event.is_read_closed() {
                return Err(Error::InputClosed);
            }
            assert!(event.is_readable());
            let source = self.token_source_map.get(&event.token().0).unwrap();
            let request = source.borrow_mut().next_request()?;
            match ret {
                None => ret = Some(request),
                Some(_) => self.result_buffer.push(request),
            };
        }
        return Ok(ret.unwrap());
    }
}
