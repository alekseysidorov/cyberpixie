use cyberpixie_core::{
    proto::types::{FirmwareInfo, ImageId},
    ExactSizeRead,
};
use cyberpixie_network::{asynch::NetworkStack, PayloadReader};
use embedded_io::asynch::Read;

pub use self::app::App;
use crate::{CyberpixieError, CyberpixieResult, Storage};

mod app;

/// Port for the client connection.
pub const CLIENT_PORT: u16 = 1800;

/// Board-specific components
///
/// Including the network stack, image and other data storage and LED strip rendering task.
pub trait Board {
    /// Type provides the internal storage functionality.
    type Storage: Storage;
    /// Type provides the network stack.
    type NetworkStack: NetworkStack;
    /// Type provides a LED strip pictures rendering task.
    type RenderTask;
    /// Returns all board components.
    ///
    /// This method brings the component ownership to the caller and can be invoked only once.
    fn take_components(&mut self) -> Option<(Self::Storage, Self::NetworkStack)>;
    /// Starts LED strip rendering task.
    ///
    /// To prevent data races, this method takes [`Self::Storage`] the entirely, making
    /// it impossible to modify it while the image rendering task is being executed.
    async fn start_rendering(
        &mut self,
        storage: Self::Storage,
        image_id: ImageId,
    ) -> CyberpixieResult<Self::RenderTask>;
    /// Stops a LED strip rendering task and returns back previously borrowed storage.
    async fn stop_rendering(&mut self, handle: Self::RenderTask)
        -> CyberpixieResult<Self::Storage>;
    /// Returns a board firmware information.
    fn firmware_info(&self) -> FirmwareInfo;

    /// Shows a debug message.
    ///
    /// Default implementation just do nothing.
    async fn show_debug_message<R: Read>(
        &self,
        mut payload: PayloadReader<R>,
    ) -> CyberpixieResult<()> {
        while payload.bytes_remaining() != 0 {
            let mut byte = [0_u8];
            payload
                .read(&mut byte)
                .await
                .map_err(CyberpixieError::network)?;
        }
        Ok(())
    }
}
