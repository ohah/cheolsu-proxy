use super::types::Platform;

pub(super) fn build_landing_html(cert_available: bool, platform: Platform) -> String {
    let android_active = matches!(platform, Platform::Android);

    let download_section = if cert_available {
        format!(
            r#"<div class="download-buttons">
              <a href="/ssl/pem" class="btn btn-ios">Download for iOS (.pem)</a>
              <a href="/ssl/der" class="btn btn-android">Download for Android (.der)</a>
              <a href="/ssl/ca.crt" class="btn btn-generic">Download (.crt)</a>
            </div>"#
        )
    } else {
        r#"<div class="alert">CA certificate is not available yet. Please start the proxy first.</div>"#.to_string()
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,user-scalable=no">
<title>Cheolsu Proxy - CA Certificate</title>
<style>
*{{margin:0;padding:0;box-sizing:border-box}}
body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;
background:#f0f2f5;color:#1a1a2e;min-height:100vh;display:flex;flex-direction:column;align-items:center;
padding:20px}}
.container{{max-width:480px;width:100%;margin:0 auto}}
.header{{text-align:center;padding:32px 0 24px}}
.header h1{{font-size:28px;font-weight:700;color:#1a1a2e;margin-bottom:4px}}
.header .subtitle{{font-size:14px;color:#666;margin-top:8px}}
.logo{{width:64px;height:64px;background:linear-gradient(135deg,#667eea 0%,#764ba2 100%);
border-radius:16px;display:flex;align-items:center;justify-content:center;margin:0 auto 16px;
font-size:28px;color:white;font-weight:bold}}
.card{{background:white;border-radius:16px;padding:24px;margin-bottom:16px;
box-shadow:0 1px 3px rgba(0,0,0,0.08),0 1px 2px rgba(0,0,0,0.06)}}
.tabs{{display:flex;gap:0;margin-bottom:20px;border-radius:10px;overflow:hidden;
background:#f0f2f5;padding:3px}}
.tab{{flex:1;padding:10px 16px;text-align:center;cursor:pointer;font-size:14px;
font-weight:600;border-radius:8px;transition:all 0.2s;color:#666;border:none;background:none}}
.tab.active{{background:white;color:#1a1a2e;box-shadow:0 1px 3px rgba(0,0,0,0.1)}}
.tab-content{{display:none}}
.tab-content.active{{display:block}}
.steps{{list-style:none;counter-reset:step}}
.steps li{{counter-increment:step;padding:12px 0 12px 44px;position:relative;
font-size:14px;line-height:1.5;border-bottom:1px solid #f0f2f5}}
.steps li:last-child{{border-bottom:none}}
.steps li::before{{content:counter(step);position:absolute;left:0;top:12px;
width:28px;height:28px;border-radius:50%;background:#667eea;color:white;
font-size:13px;font-weight:700;display:flex;align-items:center;justify-content:center}}
.steps li strong{{display:block;margin-bottom:2px}}
.download-buttons{{display:flex;flex-direction:column;gap:10px;margin-top:8px}}
.btn{{display:block;padding:14px 24px;border-radius:12px;text-decoration:none;
font-weight:600;font-size:15px;text-align:center;transition:all 0.2s}}
.btn-ios{{background:linear-gradient(135deg,#667eea 0%,#764ba2 100%);color:white}}
.btn-ios:hover{{opacity:0.9;transform:translateY(-1px)}}
.btn-android{{background:linear-gradient(135deg,#43e97b 0%,#38f9d7 100%);color:#1a1a2e}}
.btn-android:hover{{opacity:0.9;transform:translateY(-1px)}}
.btn-generic{{background:#f0f2f5;color:#1a1a2e;border:1px solid #ddd}}
.btn-generic:hover{{background:#e4e6e9;transform:translateY(-1px)}}
.alert{{padding:16px;background:#fff3cd;border-radius:10px;font-size:14px;color:#856404;text-align:center}}
.footer{{text-align:center;padding:16px 0;font-size:12px;color:#999}}
@media(max-width:360px){{
.header h1{{font-size:24px}}
.card{{padding:16px}}
}}
</style>
</head>
<body>
<div class="container">
  <div class="header">
    <div class="logo">CP</div>
    <h1>Cheolsu Proxy</h1>
    <p class="subtitle">Install the CA certificate to enable HTTPS inspection</p>
  </div>

  <div class="card">
    <h2 style="font-size:17px;margin-bottom:16px">Download Certificate</h2>
    {download_section}
  </div>

  <div class="card">
    <div class="tabs">
      <button class="tab{ios_cls}" onclick="showTab('ios')" id="tab-ios">iOS</button>
      <button class="tab{android_cls}" onclick="showTab('android')" id="tab-android">Android</button>
    </div>

    <div class="tab-content{ios_content_cls}" id="content-ios">
      <ol class="steps">
        <li><strong>Download the certificate</strong>Tap the "Download for iOS" button above. Safari will show a profile download prompt.</li>
        <li><strong>Install the profile</strong>Go to <em>Settings &gt; General &gt; VPN &amp; Device Management</em> and tap the downloaded profile to install it.</li>
        <li><strong>Trust the certificate</strong>Go to <em>Settings &gt; General &gt; About &gt; Certificate Trust Settings</em> and enable full trust for "Cheolsu Proxy CA".</li>
      </ol>
    </div>

    <div class="tab-content{android_content_cls}" id="content-android">
      <ol class="steps">
        <li><strong>Download the certificate</strong>Tap the "Download for Android" button above to download the DER file.</li>
        <li><strong>Open security settings</strong>Go to <em>Settings &gt; Security &gt; Encryption &amp; credentials &gt; Install a certificate</em>.</li>
        <li><strong>Install the certificate</strong>Select "CA certificate", then choose the downloaded file. Confirm the security warning.</li>
        <li><strong>Verify installation</strong>Go to <em>Settings &gt; Security &gt; Trusted credentials &gt; User</em> to verify "Cheolsu Proxy CA" appears.</li>
      </ol>
    </div>
  </div>

  <div class="footer">Cheolsu Proxy &middot; Serving from http://cheolsu.proxy/</div>
</div>

<script>
function showTab(name){{
  document.querySelectorAll('.tab').forEach(t=>t.classList.remove('active'));
  document.querySelectorAll('.tab-content').forEach(c=>c.classList.remove('active'));
  document.getElementById('tab-'+name).classList.add('active');
  document.getElementById('content-'+name).classList.add('active');
}}
</script>
</body>
</html>"##,
        download_section = download_section,
        ios_cls = if !android_active { " active" } else { "" },
        android_cls = if android_active { " active" } else { "" },
        ios_content_cls = if !android_active { " active" } else { "" },
        android_content_cls = if android_active { " active" } else { "" },
    )
}
