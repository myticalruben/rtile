//! Backend udev/DRM: window manager real sobre la TTY (session + DRM + GBM + libinput).
//!
//! Mínimo viable: una GPU, una salida, sin hotplug multi-GPU.

use std::os::unix::io::OwnedFd;

use smithay::{
    backend::{
        allocator::{
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            Fourcc,
        },
        drm::{DrmDevice, DrmDeviceFd, DrmNode, GbmBufferedSurface},
        egl::{EGLContext, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{gles::GlesRenderer, ImportDma},
        session::{libseat::LibSeatSession, Event as SessionEvent, Session},
        udev::{primary_gpu, UdevBackend, UdevEvent},
    },
    reexports::{
        calloop::EventLoop,
        drm::control::{connector, Device as _},
        input::Libinput,
        rustix::fs::OFlags,
        wayland_server::DisplayHandle,
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    utils::DeviceFd,
};

use crate::compositor::state::State;

/// Estado del backend udev/DRM.
pub struct UdevData {
    pub session: LibSeatSession,
    pub dh: DisplayHandle,
    pub renderer: Option<GlesRenderer>,
    pub surface: Option<GbmBufferedSurface<GbmAllocator<DrmDeviceFd>, ()>>,
    pub output: Option<Output>,
    pub seat_name: String,
}

impl UdevData {
    /// Abre la GPU (DRM + GBM + EGL) y crea la salida para el conector conectado.
    fn device_added(&mut self, node: DrmNode) -> Result<(), Box<dyn std::error::Error>> {
        let path = node.dev_path().ok_or("nodo sin ruta")?;
        let fd: OwnedFd = self.session.open(&path, OFlags::RDWR | OFlags::CLOEXEC)?;

        // Dispositivo DRM (comparte el fd con GBM/EGL vía Arc).
        let drm_fd = DrmDeviceFd::new(DeviceFd::from(fd));
        let (mut drm, _notifier) = DrmDevice::new(drm_fd.clone(), true)?;

        // Conector conectado + CRTC + modo.
        let res = drm.resource_handles()?;
        let connector = res
            .connectors()
            .iter()
            .filter_map(|h| drm.get_connector(*h, true).ok())
            .find(|c| c.state() == connector::State::Connected)
            .ok_or("sin conector conectado")?;
        let crtc = *res.crtcs().first().ok_or("sin CRTC")?;
        let mode = *connector.modes().first().ok_or("conector sin modo")?;
        let (w, h) = mode.size();
        let refresh = mode.vrefresh();

        let surface = drm.create_surface(crtc, mode, &[connector.handle()])?;

        // GBM + allocator.
        let gbm = GbmDevice::new(drm_fd)?;
        let allocator = GbmAllocator::new(gbm.clone(), GbmBufferFlags::RENDERING);

        // EGL + renderer GL.
        let egl = unsafe { EGLDisplay::new(gbm) }?;
        let context = EGLContext::new(&egl)?;
        let renderer = unsafe { GlesRenderer::new(context)? };

        // Superficie bufferizada (GBM + EGL).
        let renderer_formats = renderer.dmabuf_formats();
        let gbm_surface: GbmBufferedSurface<GbmAllocator<DrmDeviceFd>, ()> =
            GbmBufferedSurface::new(surface, allocator, &[Fourcc::Argb8888], renderer_formats)?;

        // Output (global wayland).
        let output = Output::new(
            "DRM-1".to_string(),
            PhysicalProperties {
                size: (0, 0).into(),
                subpixel: Subpixel::Unknown,
                make: "rtile".into(),
                model: "DRM".into(),
            },
        );
        let _global = output.create_global::<State>(&self.dh);
        let output_mode = Mode {
            size: (w as i32, h as i32).into(),
            refresh: refresh as i32,
        };
        output.change_current_state(Some(output_mode), None, None, Some((0, 0).into()));
        output.set_preferred(output_mode);

        self.renderer = Some(renderer);
        self.surface = Some(gbm_surface);
        self.output = Some(output);

        tracing::info!("salida DRM lista en {node:?}");
        Ok(())
    }

    /// Renderiza un frame (reutiliza `build_output_elements`).
    fn render(&mut self, state: &mut State) {
        // TODO: implementar next_buffer → render → queue_buffer → frame_submitted.
        let _ = state;
    }
}

/// Inicializa el backend udev/DRM y registra sus fuentes de eventos.
pub fn init_udev(
    event_loop: &mut EventLoop<'static, State>,
    dh: DisplayHandle,
    seat_name: String,
) -> Result<UdevData, Box<dyn std::error::Error>> {
    // 1. Session (libseat: logind/seatd).
    let (session, notifier) = LibSeatSession::new()?;

    // 2. Backend udev.
    let udev = UdevBackend::new(&seat_name)?;

    // 3. Libinput.
    let mut libinput_context = Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
        session.clone().into(),
    );
    let _ = libinput_context.udev_assign_seat(&seat_name);
    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());

    let mut data = UdevData {
        session: session.clone(),
        dh: dh.clone(),
        renderer: None,
        surface: None,
        output: None,
        seat_name: seat_name.clone(),
    };

    // 4. GPU primaria (si ya está presente).
    if let Some(node) = primary_gpu(&seat_name)?.and_then(|p| DrmNode::from_path(p).ok()) {
        let _ = data.device_added(node);
    }

    // 5. Fuentes de eventos.
    let handle = event_loop.handle();

    handle.insert_source(udev, move |event, _, state| match event {
        UdevEvent::Added { device_id, .. } => {
            if let Ok(node) = DrmNode::from_dev_id(device_id) {
                if let Some(udev) = state.udev_data.as_mut() {
                    let _ = udev.device_added(node);
                }
            }
        }
        UdevEvent::Changed { .. } => {}
        UdevEvent::Removed { .. } => {}
    })?;

    handle.insert_source(libinput_backend, move |event, _, state| {
        state.process_input_event(event);
    })?;

    handle.insert_source(notifier, move |event, _, _| match event {
        SessionEvent::PauseSession => {
            let _ = libinput_context.suspend();
        }
        SessionEvent::ActivateSession => {
            let _ = libinput_context.resume();
        }
    })?;

    Ok(data)
}
