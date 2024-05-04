//! This example showcases an interactive `Canvas` for drawing Bézier curves.
use iced::alignment;
use iced::widget::{button, container, horizontal_space, hover};
use iced::{Element, Length, Theme};

pub fn main() -> iced::Result {
    iced::program("Bezier Tool - Iced", Example::update, Example::view)
        .theme(|_| Theme::CatppuccinMocha)
        .antialiasing(true)
        .run()
}

#[derive(Default)]
struct Example {
    bezier: bezier::State,  //画布上图形的状态
    curves: Vec<bezier::Curve>,//Curve就是三个点,每三点组成一个曲线;这是曲线的集合
}

#[derive(Debug, Clone, Copy)]
enum Message {
    AddCurve(bezier::Curve),    //增加一个贝塞尔曲线
    Clear,                      //清理画布
}

impl Example {
    fn update(&mut self, message: Message) {
        match message {
            Message::AddCurve(curve) => {
                self.curves.push(curve);
                self.bezier.request_redraw();   //当新添加了一个curve时，整个画布的曲线的path就变了,需要重新绘制
            }
            Message::Clear => {
                self.bezier = bezier::State::default();
                self.curves.clear();
            }
        }
    }

    fn view(&self) -> Element<Message> {
        container(hover(
            self.bezier.view(&self.curves).map(Message::AddCurve),
            if self.curves.is_empty() {
                container(horizontal_space())
            } else {
                container(
                    button("Clear")
                        .style(button::danger)
                        .on_press(Message::Clear),
                )
                .padding(10)
                .width(Length::Fill)
                .align_x(alignment::Horizontal::Right)
            },
        ))
        .padding(20)
        .into()
    }
    /* 
    fn view(&self) -> Element<Message> {
        column![
            text("Bezier tool example").width(Length::Shrink).size(50),
            self.bezier.view(&self.curves).map(Message::AddCurve),
            button("Clear")
                .style(button::danger)
                .on_press(Message::Clear),
        ]
        .padding(20)
        .into()
    }
    */
}

mod bezier {
    use iced::mouse;
    use iced::widget::canvas::event::{self, Event};
    use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
    use iced::{Element, Length, Point, Rectangle, Renderer, Theme};

    #[derive(Default)]
    pub struct State {      
        cache: canvas::Cache,   
    }

    impl State {
        pub fn view<'a>(&'a self, curves: &'a [Curve]) -> Element<'a, Curve> {  //返回一个类型是画布的Element,这个Element的类型是Curve
            Canvas::new(Bezier {    //Bezier是一个struct，里面有一个state和一个curves,后面有对它实现canvas::Program trait,因此这个画布可以draw并且update,具体在impl Program里实现
                state: self,
                curves,
            })

            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        }

        pub fn request_redraw(&mut self) {
            self.cache.clear(); //这个cache应该是已经绘制过的曲线的集合
        }
    }

    struct Bezier<'a> { //Bezier是一个struct，里面有一个state和一个curves
        state: &'a State,
        curves: &'a [Curve],    //只有一个Curve也可以
    }

    impl<'a> canvas::Program<Curve> for Bezier<'a> {
        type State = Option<Pending>;   //Bezier的Program State定义here:Option<Pending>,它表示当前程序的状态,如果没有点击过按钮，那么它是None，如果有点击过一次按钮，那么它是Some(Pending::One),如果有点击过两次按钮，那么它是Some(Pending::Two)

        fn update(
            &self,
            state: &mut Self::State,    //这个state是可变的,因为update方法要改变当前程序的状态
            event: Event,
            bounds: Rectangle,
            cursor: mouse::Cursor,
        ) -> (event::Status, Option<Curve>) {   //update会返回Curve,把这个Curve传递给AddCurve(curve)
            let Some(cursor_position) = cursor.position_in(bounds) else {
                return (event::Status::Ignored, None);
            };

            match event {
                Event::Mouse(mouse_event) => {
                    let message = match mouse_event {
                        mouse::Event::ButtonPressed(mouse::Button::Left) => {
                            match *state {
                                None => {
                                    *state = Some(Pending::One {
                                        from: cursor_position,
                                    });

                                    None
                                }
                                Some(Pending::One { from }) => {
                                    *state = Some(Pending::Two {
                                        from,
                                        to: cursor_position,
                                    });

                                    None
                                }
                                Some(Pending::Two { from, to }) => {
                                    *state = None;

                                    Some(Curve {    //如果已经有两个点，那么就通过当前鼠标的位置创建一个曲线
                                        from,
                                        to,
                                        control: cursor_position,
                                    })
                                }
                            }
                        }
                        _ => None,  //其他鼠标事件不处理
                    };

                    (event::Status::Captured, message)  //Program返回当前状态和message，message要求增加一条曲线
                }
                _ => (event::Status::Ignored, None),    //其他事件不处理
            }
        }

        fn draw(
            &self,
            state: &Self::State,
            renderer: &Renderer,
            theme: &Theme,
            bounds: Rectangle,
            cursor: mouse::Cursor,
        ) -> Vec<Geometry> {
            let content =
                self.state.cache.draw(renderer, bounds.size(), |frame| {
                    Curve::draw_all(self.curves, frame, theme);
                    /*Curve::draw_all(self.curves, frame);*/

                    frame.stroke(   //把最外面的方框图画出来
                        &Path::rectangle(Point::ORIGIN, frame.size()),
                        Stroke::default()
                            .with_width(2.0)
                            .with_color(theme.palette().text),
                    );
                });

                if let Some(pending) = state {
                    vec![content, pending.draw(renderer, theme, bounds, cursor)]
                } else {
                    vec![content]
                }
            /* 
            if let Some(pending) = state {
                vec![content, pending.draw(renderer, bounds, cursor)]
            } else {
                vec![content]   //只有完成的曲线和方框
            }
            */
        }

        fn mouse_interaction(
            &self,
            /*If in the future you wanted to change how mouse interactions are handled based on the state of the drawing (e.g., change the cursor icon when the user is in the middle of drawing a curve), you could start using the _state parameter by removing the underscore and implementing the necessary logic. */
            _state: &Self::State,
            bounds: Rectangle,
            cursor: mouse::Cursor,
        ) -> mouse::Interaction {
            if cursor.is_over(bounds) {
                mouse::Interaction::Crosshair
            } else {
                mouse::Interaction::default()
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Curve {
        from: Point,
        to: Point,
        control: Point,
    }

    impl Curve {
        fn draw_all(curves: &[Curve], frame: &mut Frame, theme: &Theme) {
            let curves = Path::new(|p| {
                for curve in curves {
                    p.move_to(curve.from);
                    p.quadratic_curve_to(curve.control, curve.to);
                }
            });

            frame.stroke(
                &curves,
                Stroke::default()
                    .with_width(2.0)
                    .with_color(theme.palette().text),
            );
        }
        /*fn draw_all(curves: &[Curve], frame: &mut Frame) {
            let curves = Path::new(|p| {
                for curve in curves {
                    p.move_to(curve.from);
                    p.quadratic_curve_to(curve.control, curve.to);
                }
            });

            frame.stroke(&curves, Stroke::default().with_width(2.0));
        }*/
    }

    #[derive(Debug, Clone, Copy)]
    enum Pending {  //枚举出已画的point数量
        One { from: Point },
        Two { from: Point, to: Point },
    }

    impl Pending {
        fn draw(
            &self,
            renderer: &Renderer,
            theme: &Theme,
            bounds: Rectangle,
            cursor: mouse::Cursor,
        ) -> Geometry {
            let mut frame = Frame::new(renderer, bounds.size());

            if let Some(cursor_position) = cursor.position_in(bounds) {
                match *self {
                    Pending::One { from } => {
                        let line = Path::line(from, cursor_position);
                        frame.stroke(
                            &line,
                            Stroke::default()
                                .with_width(2.0)
                                .with_color(theme.palette().text),
                        );
                    }
                    Pending::Two { from, to } => {
                        let curve = Curve {
                            from,
                            to,
                            control: cursor_position,
                        };
                        Curve::draw_all(&[curve], &mut frame, theme);
                        /*Curve::draw_all(&[curve], &mut frame, theme);*/
                    }
                };
            }

            frame.into_geometry()
        }
    }
}