# Troubleshooting

This guide helps resolve common issues when using Cheolsu Proxy.

## 🚫 Proxy Won't Start

### Port Already in Use

**Symptoms**: "Port is already in use" error message

**Solution**:

1. Try changing to a different port number (e.g., 8080, 9000)
2. Change port in Settings → Proxy Port
3. Restart proxy

### Firewall Blocking

**Symptoms**: Proxy starts but connections fail

**Solution**:

1. **Windows**: Allow Cheolsu Proxy in Windows Defender Firewall
2. **macOS**: Allow in System Preferences → Security & Privacy → Firewall
3. Also allow in antivirus software settings

### Administrator Privileges Required

**Symptoms**: "Insufficient privileges" error

**Solution**:

1. Right-click Cheolsu Proxy
2. Select "Run as administrator"
3. Or log in with administrator account

## 🌐 Can't Access Websites

### Check Browser Proxy Settings

**Symptoms**: Cannot access any websites

**Solution**:

1. Verify browser proxy settings are correct
2. HTTP Proxy: `127.0.0.1:8100` (or your configured port)
3. HTTPS Proxy: `127.0.0.1:8100` (or your configured port)

### Check Internet Connection

**Symptoms**: Works without proxy but fails with proxy

**Solution**:

1. Check internet connection status
2. Verify DNS settings (recommend 8.8.8.8, 1.1.1.1)
3. Temporarily stop proxy and test direct connection

### Specific Sites Only

**Symptoms**: Some sites work, others don't

**Solution**:

1. Check if the site blocks proxies
2. Check domain filtering in proxy settings
3. Test with different browser

## 🔒 HTTPS Site Issues

### Certificate Warnings

**Symptoms**: "Your connection is not secure" warning

**Solution**:

1. Verify certificate is properly installed
2. Click "Regenerate Certificate" in Settings → Certificates
3. Restart browser

### Certificate Installation Failed

**Symptoms**: Certificate install button doesn't work

**Solution**:

1. Run Cheolsu Proxy with administrator privileges
2. Temporarily disable antivirus software
3. Use manual installation method (see Certificate Management Guide)

### Warnings Only in Some Browsers

**Symptoms**: Works in Chrome but shows warnings in Firefox

**Solution**:

1. Check certificate store for each browser
2. Firefox: Settings → Privacy & Security → Certificates
3. Chrome: Settings → Advanced → System → Manage certificates

## 📱 Mobile Connection Issues

### Mobile Can't Connect to Proxy

**Symptoms**: Mobile device can't connect after proxy setup

**Solution**:

1. Ensure PC and mobile are on same Wi-Fi
2. Verify PC's IP address is correct
3. Allow mobile IP in PC's firewall

### Mobile Certificate Installation Issues

**Symptoms**: Can't access HTTPS sites on mobile

**Solution**:

1. Transfer certificate file to mobile
2. Install certificate on mobile
3. Enable certificate trust settings

## ⚡ Performance Issues

### Slow Connection

**Symptoms**: Websites load very slowly

**Solution**:

1. Set log level to "Simple"
2. Remove unnecessary domain filtering
3. Restart proxy

### High Memory Usage

**Symptoms**: Cheolsu Proxy uses too much memory

**Solution**:

1. Clean up log files
2. Limit concurrent connections
3. Restart program

## 🔧 Settings Issues

### Settings Not Saved

**Symptoms**: Settings reset after restart

**Solution**:

1. Run Cheolsu Proxy with administrator privileges
2. Check settings file permissions
3. Check if antivirus is blocking settings files

### Auto Start Not Working

**Symptoms**: Cheolsu Proxy doesn't auto-start with computer

**Solution**:

1. Check "Auto-start on system boot" in Settings → General
2. Windows: Check Task Scheduler
3. macOS: Check Login Items

## 📊 Log Issues

### No Logs Displayed

**Symptoms**: Nothing shows in Logs tab

**Solution**:

1. Set log level to "Detailed"
2. Restart proxy
3. Access websites to generate logs

### Log Files Too Large

**Symptoms**: Log files become very large

**Solution**:

1. Set log level to "Simple"
2. Enable log file auto-rotation
3. Regularly delete log files

## 🆘 Advanced Troubleshooting

### Network Diagnostics

```bash
# Test proxy connection
curl -x http://127.0.0.1:8100 http://httpbin.org/ip

# Check port usage (Windows)
netstat -an | findstr :8100

# Check port usage (macOS)
lsof -i :8100
```

### Log File Locations

- **Windows**: `%APPDATA%\CheolsuProxy\logs\`
- **macOS**: `~/Library/Logs/CheolsuProxy/`

### Settings File Locations

- **Windows**: `%APPDATA%\CheolsuProxy\config\`
- **macOS**: `~/Library/Application Support/CheolsuProxy/config/`

## 📞 Additional Help

### If Problem Persists

1. Search for similar issues in **GitHub Issues**
2. When creating new issue, include:
   - Operating system and version
   - Cheolsu Proxy version
   - Error message screenshots
   - Steps to reproduce problem

### Useful Links

- [GitHub Issues](https://github.com/your-repo/issues)
- [FAQ](https://github.com/your-repo/wiki/FAQ)
- [Community Forum](https://github.com/your-repo/discussions)

---

**Now you can use Cheolsu Proxy freely!** If problems persist, feel free to ask for help anytime.
