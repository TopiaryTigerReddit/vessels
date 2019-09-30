use crate::input::{
    keyboard::{self as keyboard_mod, Event as KeyboardEvent},
    mouse::{self, Event as MouseEvent},
    windowing::Event as WindowingEvent,
    Event, Input as IInput,
};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use futures::{task::AtomicTask, Async, Poll, Stream};
use std::sync::Arc;

use stdweb::traits::{IEvent, IEventTarget, IKeyboardEvent};
use stdweb::web::{
    document,
    event::{
        IMouseEvent, KeyDownEvent, KeyUpEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
        MouseUpEvent, MouseWheelEvent, ResizeEvent,
    },
    window,
};

mod keyboard;

#[derive(Clone)]
pub(crate) struct Input {
    receiver: Receiver<Event>,
    sender: Sender<Event>,
    task: Arc<AtomicTask>,
}

impl Stream for Input {
    type Item = Event;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.receiver.try_recv() {
            Ok(event) => Ok(Async::Ready(Some(event))),
            Err(err) => match err {
                TryRecvError::Disconnected => panic!("Input channel disconnected!"),
                TryRecvError::Empty => {
                    self.task.register();
                    Ok(Async::NotReady)
                }
            },
        }
    }
}

impl IInput for Input {
    fn box_clone(&self) -> Box<dyn IInput> {
        Box::new(self.clone())
    }
}

impl Input {
    pub(crate) fn send(&self, event: Event) {
        if Arc::strong_count(&self.task) != 1 {
            let _ = self.sender.send(event);
            self.task.notify();
        }
    }
    pub(crate) fn new() -> Input {
        let (sender, receiver) = unbounded();
        let task = Arc::new(AtomicTask::new());
        let (resize_sender, resize_task) = (sender.clone(), task.clone());
        window().add_event_listener(move |_: ResizeEvent| {
            if Arc::strong_count(&resize_task) == 1 {
                return;
            }
            let _ = resize_sender.send(Event::Windowing(WindowingEvent::Resize));
            resize_task.notify();
        });
        let body = document().body().unwrap();
        let (mouse_up_sender, mouse_up_task) = (sender.clone(), task.clone());
        body.add_event_listener(move |event: MouseUpEvent| {
            if Arc::strong_count(&mouse_up_task) == 1 {
                return;
            }
            event.prevent_default();
            let _ = mouse_up_sender.send(Event::Mouse(MouseEvent::Up(match event.button() {
                MouseButton::Left => mouse::Button::Left,
                MouseButton::Right => mouse::Button::Right,
                MouseButton::Wheel => mouse::Button::Middle,
                MouseButton::Button4 => mouse::Button::Auxiliary(0),
                MouseButton::Button5 => mouse::Button::Auxiliary(1),
            })));
            mouse_up_task.notify();
        });
        let (mouse_down_sender, mouse_down_task) = (sender.clone(), task.clone());
        body.add_event_listener(move |event: MouseDownEvent| {
            if Arc::strong_count(&mouse_down_task) == 1 {
                return;
            }
            event.prevent_default();
            let _ = mouse_down_sender.send(Event::Mouse(MouseEvent::Down(match event.button() {
                MouseButton::Left => mouse::Button::Left,
                MouseButton::Right => mouse::Button::Right,
                MouseButton::Wheel => mouse::Button::Middle,
                MouseButton::Button4 => mouse::Button::Auxiliary(0),
                MouseButton::Button5 => mouse::Button::Auxiliary(1),
            })));
            mouse_down_task.notify();
        });
        let (mouse_move_sender, mouse_move_task) = (sender.clone(), task.clone());
        body.add_event_listener(move |event: MouseMoveEvent| {
            if Arc::strong_count(&mouse_move_task) == 1 {
                return;
            }
            event.prevent_default();
            let _ = mouse_move_sender.send(Event::Mouse(MouseEvent::Move(
                (f64::from(event.movement_x()), f64::from(event.movement_y())).into(),
            )));
            mouse_move_task.notify();
        });
        let (mouse_wheel_sender, mouse_wheel_task) = (sender.clone(), task.clone());
        body.add_event_listener(move |event: MouseWheelEvent| {
            if Arc::strong_count(&mouse_wheel_task) == 1 {
                return;
            }
            let _ = mouse_wheel_sender.send(Event::Mouse(MouseEvent::Scroll(
                (event.delta_x(), event.delta_y()).into(),
            )));
            mouse_wheel_task.notify();
        });
        let (key_down_sender, key_down_task) = (sender.clone(), task.clone());
        body.add_event_listener(move |e: KeyDownEvent| {
            if Arc::strong_count(&key_down_task) == 1 {
                return;
            }
            e.prevent_default();
            let key = e.key();
            let k = keyboard::parse_code(e.code().as_str());
            let _ = key_down_sender.send(Event::Keyboard(KeyboardEvent {
                action: keyboard_mod::Action::Down(k),
                printable: if key.len() == 1 {
                    Some(key.chars().take(1).collect::<Vec<char>>()[0])
                } else {
                    None
                },
            }));
            key_down_task.notify();
        });
        let (key_up_sender, key_up_task) = (sender.clone(), task.clone());
        body.add_event_listener(move |e: KeyUpEvent| {
            if Arc::strong_count(&key_up_task) == 1 {
                return;
            }
            e.prevent_default();
            let key = e.key();
            let k = keyboard::parse_code(e.code().as_str());
            let _ = key_up_sender.send(Event::Keyboard(KeyboardEvent {
                action: keyboard_mod::Action::Up(k),
                printable: if key.len() == 1 {
                    Some(key.chars().take(1).collect::<Vec<char>>()[0])
                } else {
                    None
                },
            }));
            key_up_task.notify();
        });
        Input {
            receiver,
            task,
            sender,
        }
    }
}
