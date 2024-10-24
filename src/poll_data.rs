use crate::{Error, Result};
use mio::{unix::SourceFd, Events, Interest, Poll, Token};
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::fd::AsRawFd;
use std::rc::Rc;

pub trait ReadData<D> {
    fn read_data(&mut self) -> Result<D>;
    fn register(&self, poll_data: &mut PollData<D>) -> Result<()>;
}

pub struct PollData<D> {
    poll: Poll,
    counter: usize,
    token_source_map: HashMap<usize, Rc<RefCell<dyn ReadData<D>>>>,
    result_buffer: std::collections::VecDeque<D>,
}

impl<D> PollData<D> {
    pub fn new(sources: &[Rc<RefCell<dyn ReadData<D>>>]) -> Result<Self> {
        let mut poll_request = Self {
            poll: Poll::new()?,
            counter: 0,
            result_buffer: vec![].into(),
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

    pub fn poll(&mut self) -> Result<D> {
        let mut ret = self.result_buffer.pop_back();
        if let Some(ret) = ret {
            return Ok(ret);
        }
        let mut events = Events::with_capacity(self.counter);
        self.poll.poll(&mut events, None)?;
        for event in events.into_iter() {
            if event.is_read_closed() {
                return Err(Error::InputClosed);
            }
            assert!(event.is_readable());
            let source = self.token_source_map.get(&event.token().0).unwrap();
            let data = source.borrow_mut().read_data()?;
            match ret {
                None => ret = Some(data),
                Some(_) => self.result_buffer.push_front(data),
            };
        }
        return Ok(ret.unwrap());
    }
}
