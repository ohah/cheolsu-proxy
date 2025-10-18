# Proxy Setup

This guide explains how to change basic settings in Cheolsu Proxy.

## 🔧 Change Port

### Why Change Port

- When default port 8100 is being used by another program
- When your company or school only allows specific ports
- When you want to use a different port for security reasons

### How to Change Port

1. Click the **Settings** button in Cheolsu Proxy
2. Enter the desired port number in the **Proxy Port** field
   - Examples: `8080`, `9000`, `3128`, etc.
3. Click the **Save** button
4. Restart the proxy

### Update Browser Settings Too

If you changed the port, you also need to update your browser settings:

1. **Chrome/Edge**: Settings → Advanced → System → Open proxy server settings
2. **Firefox**: Settings → General → Network Settings
3. Change to the new port number you set

## 🌐 Network Settings

### Local Use Only (Default)

- **Setting**: `127.0.0.1` (localhost)
- **Purpose**: Use proxy only on your computer
- **Security**: Most secure setting

### Use from Other Devices

To use the proxy from other devices (smartphone, tablet):

1. **Settings** → **Network Address** change to `0.0.0.0`
2. Set **Port** to desired number
3. **Save** and restart proxy

### Mobile Device Setup

1. Make sure PC and mobile are connected to the same Wi-Fi
2. Check PC's IP address:
   - **Windows**: Run `ipconfig` command
   - **macOS**: Run `ifconfig` command
3. On mobile device: Wi-Fi Settings → Advanced Options → Proxy Settings
4. Select **Manual** and enter **Server**: `[PC_IP]:[PORT]`

## ⚙️ Auto Start Settings

### Auto Launch on System Boot

1. **Settings** → **General** tab
2. Turn on **"Auto-start on system boot"** option
3. Click **Save** button

Now Cheolsu Proxy will automatically launch when you turn on your computer.

### Auto Start Proxy

1. **Settings** → **General** tab
2. Turn on **"Auto-start proxy when program launches"** option
3. Click **Save** button

The proxy will automatically start when the program launches.

## 📊 Log Settings

### Change Log Level

1. **Settings** → **Logs** tab
2. Select **Log Level**:
   - **Simple**: Show errors only (fast performance)
   - **Normal**: Include general information (default)
   - **Detailed**: Show all information (for debugging)

### Save Log Files

1. **Settings** → **Logs** tab
2. Turn on **"Save log files"** option
3. Select where to save log files

## 🔒 Security Settings

### Access Control

You can restrict proxy usage to specific IP addresses:

1. **Settings** → **Security** tab
2. Add IP addresses to **"Allowed IP addresses"** list
3. Examples: `192.168.1.100`, `10.0.0.0/24`, etc.

### Domain Filtering

You can block or allow specific websites:

1. **Settings** → **Security** tab
2. Add domains to **"Blocked domains"** list
3. Examples: `ads.example.com`, `*.malware.com`, etc.

## 🎨 User Interface

### Change Theme

1. **Settings** → **General** tab
2. Select **Theme**:
   - **Light**: Bright theme
   - **Dark**: Dark theme
   - **System**: Follow system settings

### Change Language

1. **Settings** → **General** tab
2. Select **Language**:
   - **한국어**
   - **English**

## 📱 Mobile Connection Setup

### Connect via QR Code

1. **Settings** → **Mobile Connection** tab
2. Click **"Generate QR Code"** button
3. Scan QR code with mobile device
4. Proxy settings will be applied automatically

### Manual Setup Information

Information needed for manual setup instead of QR code:

- **Proxy Server**: `[PC_IP]:[PORT]`
- **Certificate**: Download link provided

## ❓ Troubleshooting Settings Issues

### Can't Connect After Port Change

1. Check if firewall allows the new port
2. Check if another program is using that port
3. Try restarting the proxy

### Can't Connect from Mobile

1. Make sure PC and mobile are on the same Wi-Fi
2. Check PC's firewall settings
3. Verify PC's IP address is correct

### Auto Start Not Working

1. Try running Cheolsu Proxy with administrator privileges
2. Check if it's added to system startup programs
3. Check if antivirus software is blocking it

## 💡 Tips

### Recommended Port Numbers

- **Development**: `8000-8999`
- **General Use**: `9000-9999`
- **Corporate**: `3128`, `8080`, `8888`

### Improve Network Performance

- Setting log level to "Simple" improves performance
- Remove unnecessary domain filtering
- Regularly clean up log files

---

**Next Step**: Learn certificate setup for HTTPS sites in [Certificate Setup](./certificate-setup.md).
