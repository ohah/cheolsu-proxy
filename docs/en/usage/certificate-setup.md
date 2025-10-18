# Certificate Setup

This guide explains how to install certificates for using HTTPS sites.

## 🔐 Why Certificates Are Needed

### Using HTTPS Sites

- **HTTP Sites**: Can be used without certificates
- **HTTPS Sites**: Certificate installation required (e.g., banks, shopping sites, social media)

### Resolving Security Warnings

Without installing certificates, you'll see warnings like:

- "Your connection is not secure"
- "Certificate cannot be trusted"
- "NET::ERR_CERT_AUTHORITY_INVALID"

## 📥 Simple Installation Method

### Automatic Installation (Recommended)

1. Click **Settings** → **Certificates** in Cheolsu Proxy
2. Click the **"Install Certificate"** button
3. It will be automatically installed on your system
4. Restart your browser

### Verify Installation

1. Try accessing `https://www.google.com` in your browser
2. If it loads without warnings, the installation is complete

## 🖥️ Installation by Operating System

### Windows Installation

1. **Automatic**: Click "Install Certificate" button in Cheolsu Proxy
2. **Manual**:
   - Double-click certificate file
   - Click "Install Certificate"
   - Select "Local Machine"
   - Save to "Trusted Root Certification Authorities"

### macOS Installation

1. **Automatic**: Click "Install Certificate" button in Cheolsu Proxy
2. **Manual**:
   - Launch Keychain Access
   - Drag certificate file to System keychain
   - Double-click certificate and set to "Always Trust"

## 📱 Mobile Device Installation

### iPhone/iPad

1. Transfer certificate file via email or AirDrop
2. Tap file to install
3. Install profile in **Settings** → **General** → **VPN & Device Management**
4. Enable in **Settings** → **General** → **About** → **Certificate Trust Settings**

### Android

1. Transfer certificate file to device
2. **Settings** → **Security** → **Install Certificate**
3. Select "CA Certificate"
4. Select certificate file to install

## ✅ How to Verify Installation

### Check in Browser

1. **Chrome/Edge**: Settings → Advanced → System → Manage certificates
2. **Firefox**: Settings → Privacy & Security → Certificates → View Certificates
3. Verify "Cheolsu Proxy CA" or "Cheolsu Proxy" certificate exists

### Test Site Access

If these sites load without warnings, installation is complete:

- `https://www.google.com`
- `https://www.github.com`
- `https://www.stackoverflow.com`

## 🔄 Certificate Renewal

### Automatic Renewal

- Cheolsu Proxy automatically renews certificates
- Renewal notification appears 30 days before expiration
- Click "Renew" button when notification appears

### Manual Renewal

1. Click "Regenerate Certificate" in **Settings** → **Certificates**
2. Existing certificate is removed and new one is installed
3. Restart your browser

## 🚨 Troubleshooting

### Certificate Installation Failed

**Solution**:

1. Run Cheolsu Proxy with administrator privileges
2. Temporarily disable antivirus software
3. Try manual installation method

### Warnings Still Appear After Installation

**Solution**:

1. Clear browser cache
2. Restart browser
3. Reinstall certificate

### Warnings Only on Specific Sites

**Solution**:

1. Check if the site's certificate is correct
2. Check browser's certificate store
3. Restart proxy

## 🔒 Security Considerations

### Certificate Usage Purpose

- **Development & Testing**: Use only in local development environment
- **Personal Use**: Use only on personal computers
- **Production Prohibition**: Do not use in actual services

### Regular Management

- Renew certificates every year
- Remove certificates when not in use
- Regularly check for security updates

## ❓ Frequently Asked Questions

### Q: Is it safe to install certificates?

**A**: Yes, it's safe. Cheolsu Proxy only works locally and doesn't send data externally.

### Q: Does it work on all browsers?

**A**: Yes, it works on all major browsers including Chrome, Firefox, Safari, and Edge.

### Q: I want to remove the certificate

**A**: Click "Remove Certificate" button in Cheolsu Proxy settings, or manually remove from the operating system's certificate store.

### Q: Can I use it on company computers?

**A**: It depends on company policy. Please consult with your IT department.

## 🆘 Need Help?

If the problem persists:

1. Check the **Troubleshooting Guide**
2. Search for similar issues in **GitHub Issues**
3. Create a new issue

---

**Next Step**: Learn how to resolve common issues in [Troubleshooting Guide](./troubleshooting.md).
