use crate::{Error, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::rc::Rc;

pub trait ReadData<D> {
    fn read_data(&mut self) -> Result<D>;
    fn register(&self, poll_data: &mut PollData<D>) -> Result<()>;
}

pub struct PollData<D> {
    epoll: OwnedFd,
    counter: u64,
    id_source_map: HashMap<u64, Rc<RefCell<dyn ReadData<D>>>>,
    result_buffer: std::collections::VecDeque<D>,
}

impl<D> PollData<D> {
    pub fn new(sources: &[Rc<RefCell<dyn ReadData<D>>>]) -> Result<Self> {
        let mut poll_request = Self {
            epoll: unsafe { OwnedFd::from_raw_fd(libc::epoll_create1(0)) },
            counter: 0,
            result_buffer: vec![].into(),
            id_source_map: HashMap::new(),
        };
        for source in sources.into_iter() {
            poll_request
                .id_source_map
                .insert(poll_request.counter, Rc::clone(source));
            source.borrow().register(&mut poll_request)?;
        }
        Ok(poll_request)
    }

    pub fn register(&mut self, source: &impl AsRawFd) -> Result<()> {
        if -1
            == unsafe {
                libc::epoll_ctl(
                    self.epoll.as_raw_fd(),
                    libc::EPOLL_CTL_ADD,
                    source.as_raw_fd(),
                    &mut libc::epoll_event {
                        events: libc::EPOLLIN as u32,
                        u64: self.counter,
                    },
                )
            }
        {
            return Err(crate::Error::Io(std::io::Error::last_os_error())).unwrap();
        };
        self.counter += 1;
        Ok(())
    }

    pub fn poll(&mut self) -> Result<D> {
        let mut ret = self.result_buffer.pop_back();
        if let Some(ret) = ret {
            return Ok(ret);
        }
        let events = {
            let mut events = Vec::<libc::epoll_event>::with_capacity((self.counter * 4) as usize);
            let maxevents = events.capacity() as i32;
            let wait_return_value = unsafe {
                libc::epoll_wait(self.epoll.as_raw_fd(), events.as_mut_ptr(), maxevents, -1)
            };
            if -1 == wait_return_value {
                return Err(crate::Error::Io(std::io::Error::last_os_error())).unwrap();
            } else {
                unsafe { events.set_len(wait_return_value as usize) }
            }
            events
        };
        for event in events.into_iter() {
            if event.events & libc::EPOLLHUP as u32 != 0 {
                return Err(Error::OneOfMultipleInputClosed);
            }
            assert!(event.events & libc::EPOLLIN as u32 != 0);
            let source = self
                .id_source_map
                .get(&{
                    let u64 = event.u64;
                    u64
                })
                .unwrap();
            let data = source.borrow_mut().read_data().unwrap();
            match ret {
                None => ret = Some(data),
                Some(_) => self.result_buffer.push_front(data),
            };
        }
        return Ok(ret.unwrap());
    }
}
