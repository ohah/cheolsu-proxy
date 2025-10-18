# Basic Usage

A simple guide for first-time users of Cheolsu Proxy.

## 🚀 Getting Started

### Step 1: Launch the Program

1. Download and install Cheolsu Proxy
2. Launch the program
3. The main screen will appear

### Step 2: Start Proxy

1. Click the **"Start Proxy"** button
2. When the proxy starts, a green indicator will appear
3. It runs on `127.0.0.1:8100` by default

### Step 3: Browser Configuration

You need to configure your browser to use the proxy.

#### Chrome/Edge Setup

1. **Settings** → **Advanced** → **System** → **Open proxy server settings**
2. Turn on **Manual proxy setup**
3. **HTTP Proxy**: `127.0.0.1` **Port**: `8100`
4. **HTTPS Proxy**: `127.0.0.1` **Port**: `8100`
5. Click **Save**

#### Firefox Setup

1. **Settings** → **General** → **Network Settings**
2. Select **Manual proxy configuration**
3. **HTTP Proxy**: `127.0.0.1` **Port**: `8100`
4. **HTTPS Proxy**: `127.0.0.1` **Port**: `8100`
5. Click **OK**

## ✅ Verify Usage

### Check if Proxy is Working

1. Visit `http://httpbin.org/ip` in your browser
2. If an IP address is displayed, the proxy is working correctly
3. You can check requests in the Logs tab of Cheolsu Proxy

## 🛑 Stop Proxy

### Stop the Proxy

1. Click the **"Stop Proxy"** button in Cheolsu Proxy
2. When the proxy stops, a red indicator will appear
3. It's recommended to disable browser proxy settings

### Disable Browser Settings

- **Chrome/Edge**: Change to **"Use automatic proxy setup"** in proxy settings
- **Firefox**: Change to **"Use system proxy settings"** in proxy settings

## 🔒 Using HTTPS Sites

To use HTTPS sites, you need to install a certificate.

### Install Certificate

1. Click **Settings** → **Certificates** in Cheolsu Proxy
2. Click the **"Install Certificate"** button
3. The certificate will be installed on your system
4. Restart your browser

> **Note**: Some security warnings may appear after certificate installation. This is normal.

## 📊 Check Logs

### View Real-time Logs

1. Click the **Logs** tab in Cheolsu Proxy
2. You can view web requests and responses in real-time
3. You can see request URLs, response codes, time, etc.

## ❓ Frequently Asked Questions

### Q: Proxy won't start

**A**: Check the following:

- Check if port 8100 is being used by another program
- Check firewall settings
- Run with administrator privileges

### Q: Can't access websites

**A**: Check the following:

- Verify browser proxy settings are correct
- Check if Cheolsu Proxy is running
- Check internet connection status

### Q: Warnings appear on HTTPS sites

**A**: The certificate may not be installed properly:

- Try reinstalling the certificate
- Restart your browser
- Refer to the certificate management guide

## 🆘 Need Help?

If the problem persists:

1. Check the **Troubleshooting Guide**
2. Search for similar issues in **GitHub Issues**
3. Create a new issue

---

**Next Step**: Learn advanced settings like port changes in [Proxy Setup](./proxy-setup.md).
