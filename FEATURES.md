# New Features: Folder Monitoring & Global Speed Limits

This document describes the newly implemented features for automatic torrent monitoring and global speed limit controls.

## Feature 1: Folder Monitoring (Desktop Mode)

### Overview
Automatically monitors a specified folder for new `.torrent` files and loads them into the application. Optionally, torrents can be automatically started for seeding.

### Configuration

#### Desktop Application
1. Open the application
2. Click on **Settings** (gear icon in sidebar)
3. Navigate to the **General** tab
4. Locate the **Watch Folder** section
5. Enable the checkbox next to "Enabled"
6. Click **Browse** to select a folder to monitor
7. (Optional) Enable **"Automatically start seeding when torrents are added"**
8. **Restart the application** for changes to take effect

#### Server Mode (Docker)
The server mode already supports watch folder functionality via environment variables:

```yaml
environment:
  - WATCH_ENABLED=true          # Enable watch folder (auto-detected if /torrents volume exists)
  - WATCH_DIR=/torrents          # Directory to watch (default: /torrents)
  - WATCH_AUTO_START=true        # Auto-start faking for new torrents
volumes:
  - ./path/to/torrents:/torrents # Mount your torrent folder
```

### How It Works

1. **File Detection**: The watch service monitors the specified folder for new `.torrent` files
2. **Automatic Loading**: When a new torrent file is detected, it's automatically loaded
3. **Duplicate Prevention**: Torrents are tracked by info_hash to prevent duplicate loading
4. **Auto-Start** (optional): If enabled, loaded torrents automatically start seeding
5. **Source Tracking**: Watch folder instances are marked with a folder icon in the UI

### Technical Details

- Uses the `notify` crate for efficient file system monitoring
- Monitors create and modify events for `.torrent` files
- 500ms delay after detection ensures file is fully written
- Event-driven architecture communicates between backend and frontend
- Configuration persisted in `AppConfig` (`~/.config/rustatio/config.toml` on Linux/macOS)

### Use Cases

- **Automated Workflow**: RSS feed downloads torrents → Watch folder auto-loads them
- **Batch Processing**: Drop multiple torrents into folder for automatic loading
- **Integration**: Works with download managers, scripts, or other automation tools

---

## Feature 2: Global Speed Limits

### Overview
Set global upload and download speed limits that apply across ALL instances, not just individual torrent limits.

### Configuration

1. Open **Settings** → **General** tab
2. Locate the **Global Speed Limits** section
3. Enable the checkbox next to "Enabled"
4. Set **Upload Limit** in KB/s (0 = unlimited)
5. Set **Download Limit** in KB/s (0 = unlimited)
6. Settings save automatically

### How It Works

**Configuration**: Global limits are stored in the application configuration and persist across restarts.

**UI Integration**: 
- Available in both desktop and server modes
- Real-time configuration updates
- Visual feedback with disabled state when not enabled

**Note**: The enforcement logic (distributing bandwidth across instances) is configured but requires additional implementation for full functionality. The configuration is ready and will work once the enforcement layer is added.

### Planned Enforcement Logic

When fully implemented, the system will:
1. Calculate total requested bandwidth across all running instances
2. If total exceeds global limit, proportionally reduce each instance's rate
3. Dynamically adjust as instances start/stop
4. Maintain configured ratios between upload/download rates

### Configuration File

Settings are stored in `config.toml`:

```toml
[faker]
default_upload_rate = 50.0
default_download_rate = 100.0
global_speed_limit_enabled = true
global_upload_limit = 500.0      # KB/s
global_download_limit = 1000.0   # KB/s
```

### Use Cases

- **Bandwidth Management**: Limit total bandwidth usage across all torrents
- **ISP Compliance**: Stay within data caps or rate limits
- **Background Operation**: Run Rustatio without impacting other network activities
- **Realistic Behavior**: Emulate real torrent client behavior with global constraints

---

## Configuration File Locations

**Linux/macOS**: `~/.config/rustatio/config.toml`  
**Windows**: `%APPDATA%\rustatio\config.toml`  
**Server Mode**: `/data/config.toml` (inside container)

---

## Implementation Status

### ✅ Completed
- Watch folder backend implementation (desktop)
- Watch folder UI controls
- Global speed limit configuration
- Configuration persistence
- Event-driven architecture
- Duplicate detection

### 🚧 In Progress / Future Work
- Global speed limit enforcement logic
- Rate distribution algorithm
- Dynamic adjustment for starting/stopping instances
- Advanced rate allocation strategies
- Additional watch folder options (patterns, exclusions)

---

## Troubleshooting

### Watch Folder Not Working (Desktop)
1. Verify the folder path exists and is accessible
2. Check application has read permissions for the folder
3. Restart the application after changing settings
4. Check logs for any error messages
5. Ensure `.torrent` files have correct file extension

### Watch Folder Not Working (Server)
1. Verify volume is correctly mounted
2. Check container has permission to access mounted folder (use PUID/PGID)
3. Look for permission warnings in container logs
4. Create directory on host BEFORE starting container

### Global Speed Limits Not Applying
- Currently, limits are stored but enforcement is not yet active
- This is expected and will be implemented in a future update
- Configuration is saved correctly and ready for when enforcement is added

---

## Contributing

To contribute to these features:

1. **Watch Folder**: See `rustatio-desktop/src/watch.rs` and `rustatio-server/src/watch.rs`
2. **Global Limits**: Configuration in `rustatio-core/src/config.rs`, enforcement needed in faker logic
3. **UI**: Settings in `ui/src/components/SettingsDialog.svelte`

---

## License

These features are part of Rustatio and follow the same MIT License as the main project.
