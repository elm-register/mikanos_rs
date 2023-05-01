use alloc::rc::Rc;
use core::cell::{OnceCell, RefCell, RefMut};

use common_lib::frame_buffer::FrameBufferConfig;
use common_lib::math::size::Size;
use common_lib::math::vector::Vector2D;
use common_lib::transform::builder::Transform2DBuilder;
use kernel_lib::error::{KernelError, KernelResult, LayerReason};
use kernel_lib::gop::console::DISPLAY_BACKGROUND_COLOR;
use kernel_lib::layers::{frame_buffer_layer_transform, Layers};
use kernel_lib::layers::drawer::cursor::cursor_buffer::{CURSOR_HEIGHT, CURSOR_WIDTH};
use kernel_lib::layers::drawer::cursor::cursor_colors::CursorColors;
use kernel_lib::layers::drawer::cursor::cursor_drawer::CursorDrawer;
use kernel_lib::layers::drawer::rect_colors::RectColors;
use kernel_lib::layers::drawer::shape_drawer::ShapeDrawer;

pub static LAYERS: GlobalLayers = GlobalLayers::new_uninit();

pub struct GlobalLayers(OnceCell<Rc<RefCell<Layers>>>);


pub const BACKGROUND_LAYER_KEY: &str = "BACKGROUND";


pub const MOUSE_LAYER_KEY: &str = "MOUSE_CURSOR";


impl GlobalLayers {
    pub const fn new_uninit() -> GlobalLayers {
        Self(OnceCell::new())
    }

    pub fn init(&self, frame_buffer_config: FrameBufferConfig) -> KernelResult {
        let layers = Layers::new(frame_buffer_config);

        self.0
            .set(Rc::new(RefCell::new(layers)))
            .map_err(|_| KernelError::FailedOperateLayer(LayerReason::FailedInitialize))
    }


    pub fn layers_mut(&self) -> Rc<RefCell<Layers>> {
        Rc::clone(self.0.get().unwrap())
    }
}


unsafe impl Sync for GlobalLayers {}


pub fn init_layers(frame_buffer_config: FrameBufferConfig) -> KernelResult {
    LAYERS.init(frame_buffer_config)?;

    let layers = LAYERS.layers_mut();
    let mut layers = layers.borrow_mut();

    add_background_layer(frame_buffer_config, &mut layers);
    add_mouse_layer(frame_buffer_config, &mut layers);
    
    layers.draw_all_layer()
}


fn add_background_layer(frame_buffer_config: FrameBufferConfig, layers: &mut RefMut<Layers>) {
    let transform = frame_buffer_layer_transform(frame_buffer_config);
    let shape_drawer = ShapeDrawer::new(
        RectColors::default().change_foreground(DISPLAY_BACKGROUND_COLOR),
        frame_buffer_config.pixel_format,
    );

    layers.new_layer(BACKGROUND_LAYER_KEY, transform, shape_drawer);
}


fn add_mouse_layer(frame_buffer_config: FrameBufferConfig, layers: &mut RefMut<Layers>) {
    let transform = Transform2DBuilder::new()
        .size(Size::new(CURSOR_WIDTH, CURSOR_HEIGHT))
        .build();
    let cursor_drawer = CursorDrawer::new(
        Vector2D::unit(),
        CursorColors::default(),
        frame_buffer_config.pixel_format,
    );


    layers.new_layer(MOUSE_LAYER_KEY, transform, cursor_drawer);
}
