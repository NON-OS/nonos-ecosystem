use super::server::LOCAL_PROXY_PORT;

pub fn html(html: &str, page_url: &str) -> Vec<u8> {
    let proxy = format!("http://localhost:{}/proxy?url=", LOCAL_PROXY_PORT);
    let base = extract_origin(page_url);
    let full_base = extract_base_url(page_url);
    let mut out = inject_script(html, &proxy, &base);

    for attr in ["src", "href", "poster", "data-src", "data-lazy-src", "data-original", "data-bg", "data-image", "content", "ping", "formaction"] {
        out = rewrite_attr_full(&out, attr, &base, &full_base, &proxy);
    }

    out = rewrite_srcset(&out, &base, &proxy);
    out = rewrite_style_urls(&out, &base, &proxy);
    out = rewrite_action(&out, &base, &proxy);
    out = rewrite_meta_refresh(&out, &base, &proxy);
    out = strip_integrity(&out);
    out = strip_csp(&out);
    out = rewrite_inline_css(&out, &base, &proxy);

    out.into_bytes()
}

pub fn css(css: &str, page_url: &str) -> Vec<u8> {
    let proxy = format!("http://localhost:{}/proxy?url=", LOCAL_PROXY_PORT);
    let base = extract_origin(page_url);
    rewrite_css_content(css, &base, &proxy).into_bytes()
}

fn rewrite_css_content(css: &str, base: &str, proxy: &str) -> String {
    let mut out = css.to_string();
    out = out.replace("url(\"https://", &format!("url(\"{}https://", proxy));
    out = out.replace("url('https://", &format!("url('{}https://", proxy));
    out = out.replace("url(https://", &format!("url({}https://", proxy));
    out = out.replace("url(\"http://", &format!("url(\"{}http://", proxy));
    out = out.replace("url('http://", &format!("url('{}http://", proxy));
    out = out.replace("url(http://", &format!("url({}http://", proxy));
    out = out.replace("url(\"//", &format!("url(\"{}https://", proxy));
    out = out.replace("url('//", &format!("url('{}https://", proxy));
    out = out.replace("url(//", &format!("url({}https://", proxy));
    out = out.replace("url(\"/", &format!("url(\"{}{}/", proxy, urlencoding::encode(base)));
    out = out.replace("url('/", &format!("url('{}{}/", proxy, urlencoding::encode(base)));
    out = out.replace("url(/", &format!("url({}{}/", proxy, urlencoding::encode(base)));
    out = out.replace("@import \"https://", &format!("@import \"{}https://", proxy));
    out = out.replace("@import 'https://", &format!("@import '{}https://", proxy));
    out = out.replace("@import url(\"https://", &format!("@import url(\"{}https://", proxy));
    out
}

fn extract_origin(url: &str) -> String {
    url::Url::parse(url).ok()
        .map(|u| format!("{}://{}", u.scheme(), u.host_str().unwrap_or("")))
        .unwrap_or_default()
}

fn extract_base_url(url: &str) -> String {
    if let Ok(u) = url::Url::parse(url) {
        let path = u.path();
        if let Some(last_slash) = path.rfind('/') {
            return format!("{}://{}{}", u.scheme(), u.host_str().unwrap_or(""), &path[..=last_slash]);
        }
        return format!("{}://{}/", u.scheme(), u.host_str().unwrap_or(""));
    }
    String::new()
}

fn rewrite_attr_full(html: &str, attr: &str, base: &str, full_base: &str, proxy: &str) -> String {
    let mut out = html.to_string();
    for q in ['"', '\''] {
        out = rewrite_single_attr(&out, attr, q, base, full_base, proxy);
    }
    out
}

fn rewrite_single_attr(html: &str, attr: &str, quote: char, base: &str, full_base: &str, proxy: &str) -> String {
    let pattern = format!("{}={}", attr, quote);
    let mut result = String::new();
    let mut rest = html;

    while let Some(pos) = rest.to_lowercase().find(&pattern.to_lowercase()) {
        result.push_str(&rest[..pos]);
        let attr_start = pos + pattern.len();
        let after = &rest[attr_start..];

        if let Some(end) = after.find(quote) {
            let value = &after[..end];
            let new_value = rewrite_url_value(value, base, full_base, proxy);
            result.push_str(&rest[pos..pos + pattern.len()]);
            result.push_str(&new_value);
            rest = &rest[attr_start + end..];
        } else {
            result.push_str(&rest[pos..pos + pattern.len()]);
            rest = &rest[attr_start..];
        }
    }
    result.push_str(rest);
    result
}

fn rewrite_url_value(value: &str, base: &str, full_base: &str, proxy: &str) -> String {
    let v = value.trim();
    if v.is_empty() || v.starts_with("data:") || v.starts_with("blob:") || v.starts_with('#') || v.starts_with("javascript:") || v.contains("localhost:9060") {
        return value.to_string();
    }
    if v.starts_with("https://") || v.starts_with("http://") {
        return format!("{}{}", proxy, urlencoding::encode(v));
    }
    if v.starts_with("//") {
        return format!("{}https:{}", proxy, urlencoding::encode(v));
    }
    if v.starts_with('/') {
        return format!("{}{}{}", proxy, urlencoding::encode(base), urlencoding::encode(v));
    }
    format!("{}{}{}", proxy, urlencoding::encode(full_base), urlencoding::encode(v))
}

fn rewrite_srcset(html: &str, base: &str, proxy: &str) -> String {
    let mut result = String::new();
    let mut rest = html;

    while let Some(pos) = rest.to_lowercase().find("srcset=") {
        result.push_str(&rest[..pos]);
        let after_eq = &rest[pos + 7..];
        let quote = after_eq.chars().next().unwrap_or('"');
        if quote != '"' && quote != '\'' {
            result.push_str(&rest[pos..pos + 8]);
            rest = &rest[pos + 8..];
            continue;
        }
        let inner = &after_eq[1..];
        if let Some(end) = inner.find(quote) {
            let srcset_val = &inner[..end];
            let new_srcset = rewrite_srcset_value(srcset_val, base, proxy);
            result.push_str("srcset=");
            result.push(quote);
            result.push_str(&new_srcset);
            result.push(quote);
            rest = &inner[end + 1..];
        } else {
            result.push_str(&rest[pos..pos + 8]);
            rest = &rest[pos + 8..];
        }
    }
    result.push_str(rest);
    result
}

fn rewrite_srcset_value(srcset: &str, base: &str, proxy: &str) -> String {
    srcset.split(',')
        .map(|part| {
            let trimmed = part.trim();
            let parts: Vec<&str> = trimmed.splitn(2, char::is_whitespace).collect();
            if parts.is_empty() {
                return trimmed.to_string();
            }
            let url = parts[0];
            let descriptor = if parts.len() > 1 { parts[1].trim() } else { "" };
            let new_url = if url.starts_with("http") {
                format!("{}{}", proxy, urlencoding::encode(url))
            } else if url.starts_with("//") {
                format!("{}https:{}", proxy, urlencoding::encode(url))
            } else if url.starts_with('/') {
                format!("{}{}{}", proxy, urlencoding::encode(base), urlencoding::encode(url))
            } else {
                url.to_string()
            };
            if descriptor.is_empty() {
                new_url
            } else {
                format!("{} {}", new_url, descriptor)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn rewrite_style_urls(html: &str, base: &str, proxy: &str) -> String {
    let mut result = String::new();
    let mut rest = html;

    while let Some(pos) = rest.to_lowercase().find("style=") {
        result.push_str(&rest[..pos]);
        let after_eq = &rest[pos + 6..];
        let quote = after_eq.chars().next().unwrap_or('"');
        if quote != '"' && quote != '\'' {
            result.push_str(&rest[pos..pos + 7]);
            rest = &rest[pos + 7..];
            continue;
        }
        let inner = &after_eq[1..];
        if let Some(end) = inner.find(quote) {
            let style_val = &inner[..end];
            let new_style = rewrite_css_content(style_val, base, proxy);
            result.push_str("style=");
            result.push(quote);
            result.push_str(&new_style);
            result.push(quote);
            rest = &inner[end + 1..];
        } else {
            result.push_str(&rest[pos..pos + 7]);
            rest = &rest[pos + 7..];
        }
    }
    result.push_str(rest);
    result
}

fn rewrite_inline_css(html: &str, base: &str, proxy: &str) -> String {
    let mut result = String::new();
    let mut rest = html;

    while let Some(start) = rest.to_lowercase().find("<style") {
        let after_tag = &rest[start..];
        if let Some(tag_end) = after_tag.find('>') {
            result.push_str(&rest[..start + tag_end + 1]);
            let content_start = &rest[start + tag_end + 1..];
            if let Some(close) = content_start.to_lowercase().find("</style") {
                let css_content = &content_start[..close];
                let rewritten = rewrite_css_content(css_content, base, proxy);
                result.push_str(&rewritten);
                rest = &content_start[close..];
            } else {
                rest = content_start;
            }
        } else {
            result.push_str(&rest[..start + 6]);
            rest = &rest[start + 6..];
        }
    }
    result.push_str(rest);
    result
}

fn rewrite_action(html: &str, base: &str, proxy: &str) -> String {
    let mut out = html.to_string();
    for q in ['"', '\''] {
        let pattern = format!("action={}", q);
        let mut result = String::new();
        let mut rest = out.as_str();

        while let Some(pos) = rest.to_lowercase().find(&pattern) {
            result.push_str(&rest[..pos]);
            let after = &rest[pos + pattern.len()..];
            if let Some(end) = after.find(q) {
                let val = &after[..end];
                if val.starts_with("http") && !val.contains("localhost:9060") {
                    result.push_str(&format!("action={}{}{}", q, proxy, urlencoding::encode(val)));
                } else if val.starts_with('/') && !val.starts_with("//") {
                    result.push_str(&format!("action={}{}{}{}", q, proxy, urlencoding::encode(base), urlencoding::encode(val)));
                } else {
                    result.push_str(&rest[pos..pos + pattern.len()]);
                    result.push_str(val);
                }
                rest = &after[end..];
            } else {
                result.push_str(&rest[pos..pos + pattern.len()]);
                rest = after;
            }
        }
        result.push_str(rest);
        out = result;
    }
    out
}

fn strip_integrity(html: &str) -> String {
    let mut out = html.to_string();
    for quote in ['"', '\''] {
        let pattern = format!(" integrity={}", quote);
        while let Some(start) = out.find(&pattern) {
            if let Some(end) = out[start + pattern.len()..].find(quote) {
                out = format!("{}{}", &out[..start], &out[start + pattern.len() + end + 1..]);
            } else {
                break;
            }
        }
    }
    out
}

fn strip_csp(html: &str) -> String {
    let mut out = html.to_string();
    let patterns = [
        "content-security-policy",
        "x-content-security-policy",
        "x-webkit-csp",
    ];
    for pat in patterns {
        while let Some(start) = out.to_lowercase().find(&format!("http-equiv=\"{}\"", pat)) {
            if let Some(meta_start) = out[..start].rfind('<') {
                if let Some(meta_end) = out[start..].find('>') {
                    out = format!("{}{}", &out[..meta_start], &out[start + meta_end + 1..]);
                    continue;
                }
            }
            break;
        }
    }
    out
}

fn rewrite_meta_refresh(html: &str, base: &str, proxy: &str) -> String {
    let mut out = html.to_string();
    let lower = out.to_lowercase();

    if let Some(start) = lower.find("http-equiv=\"refresh\"") {
        if let Some(meta_start) = lower[..start].rfind('<') {
            let meta_tag = &out[meta_start..];
            if let Some(content_start) = meta_tag.to_lowercase().find("content=\"") {
                let content_begin = meta_start + content_start + 9;
                if let Some(content_end) = out[content_begin..].find('"') {
                    let content_val = &out[content_begin..content_begin + content_end];
                    if let Some(url_pos) = content_val.to_lowercase().find("url=") {
                        let url_start = url_pos + 4;
                        let url = content_val[url_start..].trim();
                        if url.starts_with("http") && !url.contains("localhost:9060") {
                            let new_url = format!("{}{}", proxy, urlencoding::encode(url));
                            let new_content = format!("{}url={}", &content_val[..url_start], new_url);
                            out = format!("{}{}{}", &out[..content_begin], new_content, &out[content_begin + content_end..]);
                        } else if url.starts_with('/') && !url.starts_with("//") {
                            let new_url = format!("{}{}{}", proxy, urlencoding::encode(base), urlencoding::encode(url));
                            let new_content = format!("{}url={}", &content_val[..url_start], new_url);
                            out = format!("{}{}{}", &out[..content_begin], new_content, &out[content_begin + content_end..]);
                        }
                    }
                }
            }
        }
    }
    out
}

fn inject_script(html: &str, proxy: &str, origin: &str) -> String {
    let script = format!(r#"<script>
(function(){{
var P='{}',O='{}';
var B=function(){{throw new Error('Blocked for privacy')}};
Object.defineProperty(window,'RTCPeerConnection',{{value:B,writable:false,configurable:false}});
Object.defineProperty(window,'webkitRTCPeerConnection',{{value:B,writable:false,configurable:false}});
Object.defineProperty(window,'mozRTCPeerConnection',{{value:B,writable:false,configurable:false}});
Object.defineProperty(window,'WebSocket',{{value:B,writable:false,configurable:false}});
Object.defineProperty(window,'EventSource',{{value:B,writable:false,configurable:false}});
if(navigator.serviceWorker){{Object.defineProperty(navigator.serviceWorker,'register',{{value:function(){{return Promise.reject(new Error('Service Workers disabled'))}},writable:false}})}}
if(navigator.sendBeacon){{Object.defineProperty(navigator,'sendBeacon',{{value:function(){{return false}},writable:false,configurable:false}})}}
function px(u){{
if(!u||typeof u!=='string')return u;
if(u.startsWith('data:')||u.startsWith('blob:')||u.startsWith('javascript:')||u.includes('localhost:9060'))return u;
try{{var url=u.startsWith('http')?u:new URL(u,O).href;return P+encodeURIComponent(url)}}catch(e){{return u}}
}}
var _fetch=window.fetch;
var pxFetch=function(r,o){{
var url=typeof r==='string'?r:(r&&r.url?r.url:r);
var pu=px(url);
if(typeof r==='string')return _fetch(pu,o);
if(r&&typeof r==='object'){{var nr=new Request(pu,r);return _fetch(nr,o)}}
return _fetch(r,o)
}};
Object.defineProperty(window,'fetch',{{value:pxFetch,writable:false,configurable:false}});
var _xhr=XMLHttpRequest.prototype.open;
XMLHttpRequest.prototype.open=function(m,u,a,us,p){{return _xhr.call(this,m,px(u),a!==false,us,p)}};
Object.defineProperty(XMLHttpRequest.prototype,'open',{{value:XMLHttpRequest.prototype.open,writable:false,configurable:false}});
var _img=window.Image;
var PxImage=function(w,h){{var i=new _img(w,h);var _src=Object.getOwnPropertyDescriptor(HTMLImageElement.prototype,'src');Object.defineProperty(i,'src',{{set:function(v){{_src.set.call(this,px(v))}},get:function(){{return _src.get.call(this)}}}});return i}};
Object.defineProperty(window,'Image',{{value:PxImage,writable:false,configurable:false}});
if(window.Worker){{var _Worker=window.Worker;var PxWorker=function(u,o){{return new _Worker(px(u),o)}};Object.defineProperty(window,'Worker',{{value:PxWorker,writable:false,configurable:false}})}}
if(window.SharedWorker){{var _SW=window.SharedWorker;var PxSW=function(u,o){{return new _SW(px(u),o)}};Object.defineProperty(window,'SharedWorker',{{value:PxSW,writable:false,configurable:false}})}}
var _sS=Element.prototype.setAttribute;
Element.prototype.setAttribute=function(n,v){{
if((n==='src'||n==='href'||n==='poster'||n==='data-src'||n==='srcset'||n==='ping'||n==='formaction')&&typeof v==='string'){{
if(n==='srcset'){{v=v.split(',').map(function(p){{var ps=p.trim().split(/\s+/);if(ps[0])ps[0]=px(ps[0]);return ps.join(' ')}}).join(', ')}}
else{{v=px(v)}}
}}
return _sS.call(this,n,v)
}};
document.addEventListener('click',function(e){{
var t=e.target;while(t&&t.tagName!=='A')t=t.parentElement;
if(t&&t.href&&!t.href.startsWith('javascript:')&&!t.href.startsWith('#')){{
e.preventDefault();e.stopPropagation();
var h=t.href;
if(h.includes('localhost:9060')){{var m=h.match(/proxy\?url=(.+)$/);if(m)h=decodeURIComponent(m[1])}}
window.parent.postMessage({{type:'navigate',url:h}},'*')
}}
}},true);
document.addEventListener('submit',function(e){{
var f=e.target;if(f.tagName==='FORM'){{
e.preventDefault();
var fd=new FormData(f);
var qs=new URLSearchParams(fd).toString();
var u=f.action||O;
if(u.includes('localhost:9060')){{var m=u.match(/proxy\?url=(.+)$/);if(m)u=decodeURIComponent(m[1])}}
if(f.method&&f.method.toLowerCase()==='post'){{
fetch(P+encodeURIComponent(u),{{method:'POST',body:fd}}).then(function(r){{return r.text()}}).then(function(h){{document.open();document.write(h);document.close()}}).catch(function(){{}})
}}else{{
if(qs)u+=(u.includes('?')?'&':'?')+qs;
window.parent.postMessage({{type:'navigate',url:u}},'*')
}}
}}
}},true);
var obs=new MutationObserver(function(ms){{
ms.forEach(function(m){{
m.addedNodes.forEach(function(n){{
if(n.nodeType===1){{
['src','href','poster','data-src','ping','formaction'].forEach(function(a){{
var v=n.getAttribute&&n.getAttribute(a);
if(v&&!v.includes('localhost:9060')&&(v.startsWith('http')||v.startsWith('//'))){{
n.setAttribute(a,px(v))
}}
}});
if(n.querySelectorAll){{
n.querySelectorAll('[src],[href],[poster],[data-src],[ping],[formaction]').forEach(function(el){{
['src','href','poster','data-src','ping','formaction'].forEach(function(a){{
var v=el.getAttribute(a);
if(v&&!v.includes('localhost:9060')&&(v.startsWith('http')||v.startsWith('//'))){{
el.setAttribute(a,px(v))
}}
}})
}})
}}
}}
}})
}})
}});
obs.observe(document.documentElement,{{childList:true,subtree:true}});
}})();
</script>"#, proxy, origin);

    if let Some(pos) = html.find("<!") {
        if html[pos..].to_lowercase().starts_with("<!doctype") {
            if let Some(end) = html[pos..].find('>') {
                return format!("{}{}{}", &html[..pos + end + 1], script, &html[pos + end + 1..]);
            }
        }
    }
    format!("{}{}", script, html)
}
