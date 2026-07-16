# Tailwind offline = همان Play CDN، فایل محلی

این پوشه فقط راهنماست. فایل واقعی اینجاست:

| فایل | نقش |
|------|-----|
| `dns-cli/static/tailwindcss.js` | دانلود `https://cdn.tailwindcss.com` (آفلاین) |
| `dns-cli/static/site.css` | استایل دکمه‌ها/پنل |
| `dns-cli/static/index.html` | `<script src="/tailwindcss.js"></script>` |

مثل Bootstrap: یک فایل آماده را لینک می‌کنی؛ بیلد Node لازم نیست.

به‌روزرسانی از CDN رسمی:

```powershell
Invoke-WebRequest https://cdn.tailwindcss.com -OutFile dns-cli\static\tailwindcss.js
```
