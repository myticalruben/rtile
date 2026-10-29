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
        renderer::{damage::OutputDamageTracker, gles::GlesRenderer, Bind, ImportDma},
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
    pub damage_tracker: Option<OutputDamageTracker>,
    pub seat_name: String,
}

impl UdevData {
    /// Abre la GPU (DRM + GBM + EGL) y crea la salida para el conector conectado.
    fn device_added(&mut self, node: DrmNode) -> Result<(), Box<dyn std::error::Error>> {
        let path = node.dev_path().ok_or("nodo sin ruta")?;
        let fd: OwnedFd = self.session.open(&path, OFlags::RDWR | OFlags::CLOEXEC)?;

        let drm_fd = DrmDeviceFd::new(DeviceFd::from(fd));
        let (mut drm, _notifier) = DrmDevice::new(drm_fd.clone(), true)?;

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

        let gbm = GbmDevice::new(drm_fd)?;
        let allocator = GbmAllocator::new(gbm.clone(), GbmBufferFlags::RENDERING);

        let egl = unsafe { EGLDisplay::new(gbm) }?;
        let context = EGLContext::new(&egl)?;
        let renderer = unsafe { GlesRenderer::new(context)? };

        let renderer_formats = renderer.dmabuf_formats();
        let gbm_surface: GbmBufferedSurface<GbmAllocator<DrmDeviceFd>, ()> =
            GbmBufferedSurface::new(surface, allocator, &[Fourcc::Argb8888], renderer_formats)?;

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

        self.damage_tracker = Some(OutputDamageTracker::from_output(&output));
        self.renderer = Some(renderer);
        self.surface = Some(gbm_surface);
        self.output = Some(output);

        tracing::info!("salida DRM lista en {node:?}");
        Ok(())
    }
}

impl State {
    /// Renderiza un frame en la salida DRM.
    pub fn udev_render(&mut self) {
        let Some(udev) = self.udev_data.as_mut() else {
            return;
        };
        let Some(renderer) = udev.renderer.as_mut() else {
            return;
        };
        let Some(surface) = udev.surface.as_mut() else {
            return;
        };
        let Some(output) = udev.output.as_ref() else {
            return;
        };
        let Some(damage_tracker) = udev.damage_tracker.as_mut() else {
            return;
        };

        let scale = output.current_scale().fractional_scale();
        let elements = super::render::build_output_elements(
            &self.space,
            self.focused_client,
            &self.config.appearance,
            &*smithay::desktop::layer_map_for_output(output),
            renderer,
            scale,
        );
        let clear_color = self.config.appearance.background.to_array();

        let (mut dmabuf, age) = match surface.next_buffer() {
            Ok(x) => x,
            Err(e) => {
                tracing::warn!("next_buffer: {e:?}");
                return;
            }
        };

        let mut target = match renderer.bind(&mut dmabuf) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("bind: {e:?}");
                return;
            }
        };

        if let Err(e) = damage_tracker.render_output(renderer, &mut target, age as usize, &elements, clear_color)
        {
            tracing::warn!("render_output: {e:?}");
        }

        let geo = self.space.output_geometry(output).unwrap_or_default();
        let damage = vec![geo.to_physical(1)];
        let _ = surface.queue_buffer(None, Some(damage), ());
    }
}

/// Inicializa el backend udev/DRM y registra sus fuentes de eventos.
pub fn init_udev(
    event_loop: &mut EventLoop<'static, State>,
    dh: DisplayHandle,
    seat_name: String,
) -> Result<UdevData, Box<dyn std::error::Error>> {
    let (session, notifier) = LibSeatSession::new()?;

    let udev = UdevBackend::new(&seat_name)?;

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
        damage_tracker: None,
        seat_name: seat_name.clone(),
    };

    // GPU primaria (si ya está presente).
    if let Some(node) = primary_gpu(&seat_name)?.and_then(|p| DrmNode::from_path(p).ok()) {
        if let Err(e) = data.device_added(node) {
            tracing::warn!("no se pudo abrir la GPU: {e}");
        }
    }

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

    // Primer frame tras arrancar.
    handle.insert_idle(|state| {
        state.udev_render();
    });

    Ok(data)
}
