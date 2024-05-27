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
            //这个curve是由Bezier::update返回的(captured, some(curve))中的curve
            Message::AddCurve(curve) => {
                self.curves.push(curve);
                //增加一个curve后,清除cache,重新绘制,绘制完后再存到cache,runtime就不会重复draw,使用cache里的geometry render就好了,提高性能
                self.bezier.request_redraw();   
            }
            Message::Clear => {
                self.bezier = bezier::State::default();
                self.curves.clear();
            }
        }
    }

    fn view(&self) -> Element<Message> {
        container(hover(
            //实际执行State的view:使用Example::curves作为参数创建canvas Bezier program,用Bezier的draw方法画出,把由Bezier::update返回的(captured, some(curve))中的curve做为AddCurve(curve)的curve返回给Example::update
            //直观一点解释是显示使用Bezier的draw,消息传递是把Bezier::update映射成Example::update
            self.bezier.view(&self.curves).map(Message::AddCurve),//view返回Element<'a, Curve> ,其中的curve随着程序的运行通过Bezier::update不断增加
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
        //可以利用&Example
        pub fn view<'a>(&'a self, curves: &'a [Curve]) -> Element<'a, Curve> {  //返回一个元素
            Canvas::new(Bezier {    //Bezier是一个struct，里面有一个state和一个curves,后面有对它实现canvas::Program trait,因此这个画布可以draw并且update,具体在impl Program里实现
                state: self,
                curves,
            })

            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        }

        pub fn request_redraw(&mut self) {
            self.cache.clear(); //这个cache应该是在绘制曲线的时候保存状态的,例如一个点,两个点，三个点，三个点后就是curve了,清理一下
        }
    }
    //定义Bezier的fields是引用,这样就可以使用Example的curves
    struct Bezier<'a> { //Bezier是一个struct，里面有一个state和一个curves
        state: &'a State,   //引用当前画布cache状态
        curves: &'a [Curve],    //只有一个Curve也可以,这个curves是已成型曲线的集合
    }

    impl<'a> canvas::Program<Curve> for Bezier<'a> {
        type State = Option<Pending>;   //Bezier的Program State定义here:Option<Pending>,它表示当前程序的状态,如果没有点击过鼠标，那么它是None，如果有点击过一次，那么它是Some(Pending::One),如果有点击过两次，那么它是Some(Pending::Two),再点就是curve了

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
                                    //如果有一个鼠标左键事件,并且当前Pending是None，那么就把它设置成Some(Pending::One)，这是bezier第一个点的位置
                                    *state = Some(Pending::One {
                                        from: cursor_position,
                                    });
                                    //由于还没有形成一个bezier曲线，所以返回None
                                    None
                                }
                                //如果有一个鼠标左键事件,并且当前Pending是Some(Pending::One),那么就把它设置成Some(Pending::Two)，这是bezier第二个点的位置
                                Some(Pending::One { from }) => {
                                    *state = Some(Pending::Two {
                                        from,
                                        to: cursor_position,
                                    });
                                    //由于还没有形成一个bezier曲线，所以返回None
                                    None
                                }
                                //如果有一个鼠标左键事件,并且当前Pending是Some(Pending::Two),那么就重置state，把它设置成None，这是bezier第三个点的位置,curve完成
                                Some(Pending::Two { from, to }) => {
                                    *state = None;
                                    //返回一个Curve实例
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
        //返回geometry,供view去render
        fn draw(
            &self,
            state: &Self::State,
            renderer: &Renderer,
            theme: &Theme,
            bounds: Rectangle,
            cursor: mouse::Cursor,
        ) -> Vec<Geometry> {
            //使用cache的draw方法绘制,这样可以减少重绘的次数,提高性能
            let content =
                self.state.cache.draw(renderer, bounds.size(), |frame| {
                    Curve::draw_all(self.curves, frame, theme);
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
        //这里的frame需要用到Program::draw里的cache的draw方法，所以是&mut frame
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