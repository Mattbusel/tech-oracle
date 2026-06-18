//! Render the public (delayed) page from the revealed archive using minijinja.
//! The template file is embedded at compile time, so the binary stays
//! self-contained. This module knows nothing about payments; it only renders
//! whatever the caller decided is public, plus static subscribe links.

use crate::model::Prediction;
use chrono::{NaiveDate, Utc};
use std::collections::{BTreeMap, HashMap};

// IndexNow ownership key (not secret; proves we control the host via a key file).
const INDEXNOW_KEY: &str = "0f9c2a7b5e3d4148a6c1b2e3f4a5d6c7";

// Shared card-art engine, shipped as a static asset so the Hall of Champions can
// render the exact same generative crests as the live floor. Mirrors the floor's
// inline art (kept in sync by hand; both live in this file). Namespaced under
// window.CardArt so it never clashes with the floor's own copy.
const CARD_ART_JS: &str = r##"(function(){
var FIN={shiny:{n:'SHINY',c:'#cfd8e3',m:1.5},gold:{n:'GOLD',c:'#ffd56b',m:2.2},emerald:{n:'EMERALD',c:'#4fe0a0',m:3.2},sapphire:{n:'SAPPHIRE',c:'#5fb8ff',m:4.2},diamond:{n:'DIAMOND',c:'#d8f0ff',m:7}};
function hashStr(s){var h=2166136261>>>0;s=s||'';for(var i=0;i<s.length;i++){h^=s.charCodeAt(i);h=Math.imul(h,16777619);}return h>>>0;}
function mulberry(a){return function(){a|=0;a=a+0x6D2B79F5|0;var t=Math.imul(a^a>>>15,1|a);t=t+Math.imul(t^t>>>7,61|t)^t;return((t^t>>>14)>>>0)/4294967296;};}
function ascended(c){return !!(c&&c.resolved&&(c.resolved.ascended||(Math.max(c.resolved.peak||0,c.resolved.finalBank||0,c.resolved.biggestWin||0)>=1e13)));}
function hsl(h,s,l){return 'hsl('+(((h%360)+360)%360)+','+s+'%,'+l+'%)';}
function artPalette(card,rarity,rng){var hue=Math.floor((card.risk!=null?card.risk:rng())*360),acc=(hue+90+Math.floor(rng()*150))%360;var p={bg0:hsl(hue,40,9),bg1:'#06080b',ink:hsl(acc,72,64),ink2:hsl(hue,58,52),glow:0,metal:null};if(rarity==='rare'){p.bg0=hsl(hue,52,12);p.ink=hsl(acc,84,67);p.glow=8;}if(rarity==='legend'){p.bg0='#2a1e05';p.bg1='#0a0a08';p.ink='#ffd56b';p.ink2='#caa64a';p.glow=12;p.metal='gold';}if(card.finish&&FIN[card.finish]){p.ink=FIN[card.finish].c;p.ink2=FIN[card.finish].c;p.glow=Math.max(p.glow,9);}return p;}
function artBg(ctx,W,H,p,rng){var g=ctx.createRadialGradient(W*0.5,H*0.42,2,W*0.5,H*0.5,Math.max(W,H)*0.78);g.addColorStop(0,p.bg0);g.addColorStop(1,p.bg1);ctx.fillStyle=g;ctx.fillRect(0,0,W,H);if(p.metal==='gold'){var go=ctx.createRadialGradient(W/2,H*0.46,1,W/2,H*0.46,Math.max(W,H)*0.5);go.addColorStop(0,'rgba(255,213,107,0.18)');go.addColorStop(1,'rgba(255,213,107,0)');ctx.fillStyle=go;ctx.fillRect(0,0,W,H);}ctx.fillStyle='rgba(255,255,255,0.05)';for(var i=0;i<W*H/900;i++)ctx.fillRect((rng()*W)|0,(rng()*H)|0,1,1);var v=ctx.createRadialGradient(W/2,H/2,Math.min(W,H)*0.18,W/2,H/2,Math.max(W,H)*0.72);v.addColorStop(0,'rgba(0,0,0,0)');v.addColorStop(1,'rgba(0,0,0,0.55)');ctx.fillStyle=v;ctx.fillRect(0,0,W,H);}
function sysOrbital(ctx,W,H,p,rng){var cx=W/2,cy=H/2,R=Math.min(W,H)*0.46,rings=2+(rng()*3|0),lw=Math.max(0.8,W/200);ctx.lineWidth=lw;for(var r=0;r<rings;r++){var rr=R*(0.32+0.68*(r+1)/rings),sq=0.5+rng()*0.5,rot=rng()*3;ctx.strokeStyle=p.ink2;ctx.globalAlpha=0.3;ctx.beginPath();ctx.ellipse(cx,cy,rr,rr*sq,rot,0,7);ctx.stroke();var bodies=1+(rng()*3|0);for(var b=0;b<bodies;b++){var a=rng()*7,x=cx+Math.cos(a)*rr*Math.cos(rot)-Math.sin(a)*rr*sq*Math.sin(rot),y=cy+Math.cos(a)*rr*Math.sin(rot)+Math.sin(a)*rr*sq*Math.cos(rot);ctx.globalAlpha=0.92;ctx.fillStyle=p.ink;ctx.shadowColor=p.ink;ctx.shadowBlur=p.glow;ctx.beginPath();ctx.arc(x,y,lw*(1.4+rng()*2),0,7);ctx.fill();}}ctx.shadowBlur=0;ctx.globalAlpha=1;ctx.fillStyle=p.ink;ctx.beginPath();ctx.arc(cx,cy,lw*2.3,0,7);ctx.fill();}
function sysMandala(ctx,W,H,p,rng){var cx=W/2,cy=H/2,R=Math.min(W,H)*0.47,k=5+(rng()*8|0),arms=2+(rng()*3|0),lw=Math.max(0.8,W/230),i;ctx.lineWidth=lw;ctx.strokeStyle=p.ink;ctx.fillStyle=p.ink;ctx.shadowColor=p.ink;ctx.shadowBlur=p.glow*0.6;var pts=[];for(i=0;i<arms;i++)pts.push([R*(0.25+rng()*0.75),(rng()-0.5)*0.6,lw*(1+rng()*2)]);for(var s=0;s<k;s++){ctx.save();ctx.translate(cx,cy);ctx.rotate(s/k*Math.PI*2);ctx.globalAlpha=0.85;ctx.beginPath();ctx.moveTo(0,0);for(i=0;i<pts.length;i++)ctx.lineTo(Math.cos(pts[i][1])*pts[i][0],Math.sin(pts[i][1])*pts[i][0]);ctx.stroke();for(i=0;i<pts.length;i++){ctx.beginPath();ctx.arc(Math.cos(pts[i][1])*pts[i][0],Math.sin(pts[i][1])*pts[i][0],pts[i][2],0,7);ctx.fill();}ctx.restore();}ctx.shadowBlur=0;ctx.globalAlpha=1;}
function sysConstellation(ctx,W,H,p,rng){var n=6+(rng()*10|0),ns=[],i,j,lw=Math.max(0.7,W/240);for(i=0;i<n;i++)ns.push([W*0.12+rng()*W*0.76,H*0.12+rng()*H*0.76,lw*(1+rng()*2.2)]);ctx.strokeStyle=p.ink2;ctx.globalAlpha=0.25;ctx.lineWidth=lw*0.7;var d2=(W*0.34)*(W*0.34);for(i=0;i<n;i++)for(j=i+1;j<n;j++){var dx=ns[i][0]-ns[j][0],dy=ns[i][1]-ns[j][1];if(dx*dx+dy*dy<d2){ctx.beginPath();ctx.moveTo(ns[i][0],ns[i][1]);ctx.lineTo(ns[j][0],ns[j][1]);ctx.stroke();}}ctx.globalAlpha=1;ctx.fillStyle=p.ink;ctx.shadowColor=p.ink;ctx.shadowBlur=p.glow;for(i=0;i<n;i++){ctx.beginPath();ctx.arc(ns[i][0],ns[i][1],ns[i][2],0,7);ctx.fill();}ctx.shadowBlur=0;}
function sysCircuit(ctx,W,H,p,rng){var step=Math.max(10,W/9),lw=Math.max(0.8,W/210),x,y;ctx.strokeStyle=p.ink2;ctx.lineWidth=lw;ctx.globalAlpha=0.5;for(x=step;x<W;x+=step){var turn=step+((rng()*Math.max(1,(H/step)|0))|0)*step;ctx.beginPath();ctx.moveTo(x,0);ctx.lineTo(x,turn);ctx.lineTo(x+(rng()<0.5?step:-step),turn);ctx.stroke();}ctx.globalAlpha=0.95;ctx.fillStyle=p.ink;ctx.shadowColor=p.ink;ctx.shadowBlur=p.glow*0.6;for(x=step;x<W;x+=step)for(y=step;y<H;y+=step)if(rng()<0.16){ctx.beginPath();ctx.arc(x,y,lw*1.7,0,7);ctx.fill();}ctx.shadowBlur=0;ctx.globalAlpha=1;}
function sysWave(ctx,W,H,p,rng){var cx=W*(0.3+rng()*0.4),cy=H*(0.3+rng()*0.4),rings=5+(rng()*8|0),gap=Math.min(W,H)/(rings*1.5),lw=Math.max(0.7,W/260),i;ctx.lineWidth=lw;ctx.strokeStyle=p.ink;for(i=1;i<=rings;i++){ctx.globalAlpha=0.5*(1-i/rings)+0.12;ctx.beginPath();ctx.arc(cx,cy,i*gap,0,7);ctx.stroke();}var cx2=W-cx;ctx.strokeStyle=p.ink2;for(i=1;i<=rings;i++){ctx.globalAlpha=0.4*(1-i/rings)+0.1;ctx.beginPath();ctx.arc(cx2,cy,i*gap,0,7);ctx.stroke();}ctx.globalAlpha=1;}
function sysShards(ctx,W,H,p,rng){var cx=W/2,cy=H/2,R=Math.min(W,H)*0.52,n=4+(rng()*6|0),lw=Math.max(0.8,W/220),i,ang=[];for(i=0;i<n;i++)ang.push(rng()*Math.PI*2);ang.sort(function(a,b){return a-b;});for(i=0;i<n;i++){var a0=ang[i],a1=(i+1<n?ang[i+1]:ang[0]+Math.PI*2),r0=R*(0.4+rng()*0.6),r1=R*(0.4+rng()*0.6);ctx.beginPath();ctx.moveTo(cx,cy);ctx.lineTo(cx+Math.cos(a0)*r0,cy+Math.sin(a0)*r0);ctx.lineTo(cx+Math.cos(a1)*r1,cy+Math.sin(a1)*r1);ctx.closePath();ctx.globalAlpha=0.16+rng()*0.24;ctx.fillStyle=(i%2?p.ink:p.ink2);ctx.fill();ctx.globalAlpha=0.6;ctx.lineWidth=lw*0.8;ctx.strokeStyle=p.ink;ctx.stroke();}ctx.globalAlpha=1;ctx.fillStyle=p.ink;ctx.shadowColor=p.ink;ctx.shadowBlur=p.glow;ctx.beginPath();ctx.arc(cx,cy,lw*1.8,0,7);ctx.fill();ctx.shadowBlur=0;}
function foil(ctx,W,H,str,rng){ctx.save();ctx.globalCompositeOperation='lighter';var g=ctx.createLinearGradient(0,0,W,H);g.addColorStop(0,'#ff4d6d');g.addColorStop(0.25,'#ffd56b');g.addColorStop(0.5,'#4fe0a0');g.addColorStop(0.75,'#5fb8ff');g.addColorStop(1,'#c77dff');ctx.globalAlpha=0.05+0.10*str;ctx.fillStyle=g;ctx.fillRect(0,0,W,H);var off=0.2+rng()*0.6,s=ctx.createLinearGradient(0,0,W,H);s.addColorStop(Math.max(0,off-0.14),'rgba(255,255,255,0)');s.addColorStop(off,'rgba(255,255,255,0.55)');s.addColorStop(Math.min(1,off+0.14),'rgba(255,255,255,0)');ctx.globalAlpha=0.12+0.22*str;ctx.fillStyle=s;ctx.fillRect(0,0,W,H);ctx.restore();}
function glitch(ctx,W,H,rng){try{var slabs=4+(rng()*5|0);for(var i=0;i<slabs;i++){var sy=(rng()*H)|0,sh=Math.max(2,(rng()*H*0.16)|0);if(sy+sh>H)sh=H-sy;if(sh<1)continue;var dx=((rng()-0.5)*W*0.22)|0;var img=ctx.getImageData(0,sy,W,sh);ctx.putImageData(img,dx,sy);}}catch(e){}ctx.save();for(var t=0;t<3;t++){var ty=(rng()*H)|0;ctx.globalAlpha=0.5;ctx.fillStyle=['#ff2d55','#5fb8ff','#4fe0a0'][t%3];ctx.fillRect(0,ty,W,1+(rng()*2|0));}ctx.globalAlpha=0.12;ctx.fillStyle='#000';for(var y=0;y<H;y+=3)ctx.fillRect(0,y,W,1);ctx.restore();}
var ART_SYS=[sysOrbital,sysMandala,sysConstellation,sysCircuit,sysWave,sysShards];
function drawEmblem(cv,card,rarity){var ctx=cv.getContext('2d'),W=cv.width,H=cv.height;if(!ctx||!W||!H)return;var key=card.code||card.name||'',rng=mulberry(hashStr(key+'|art')),asc=ascended(card);var p=artPalette(card,rarity,rng);artBg(ctx,W,H,p,rng);var pick=hashStr(key+'|sys')%ART_SYS.length;if(card.finish==='diamond'||card.finish==='sapphire')pick=5;ctx.save();ART_SYS[pick](ctx,W,H,p,rng);ctx.restore();if(card.finish&&FIN[card.finish])foil(ctx,W,H,(FIN[card.finish].m||1)/7,rng);if(asc){ctx.save();glitch(ctx,W,H,rng);ctx.restore();}var fw=Math.max(1,W/120);ctx.lineWidth=fw;ctx.globalAlpha=0.8;ctx.strokeStyle=asc?'#d8f0ff':(rarity==='legend'?'#ffd56b':(rarity==='rare'?'#6fb8ff':p.ink2));ctx.strokeRect(fw,fw,W-fw*2,H-fw*2);ctx.globalAlpha=1;}
window.CardArt={drawEmblem:drawEmblem,FIN:FIN,ascended:ascended};
})();"##;

// THE HALL OF CHAMPIONS: the engine's authoritative, verifiable champions, plus a
// verifier. There is no backend, so the engine itself is the notary: it publishes
// api/certified.json in CI, and a card is "verified" only if its organism appears
// there. Placeholders __SITE__ / __SITEJS__ are substituted at render time (no
// format! so the JS/CSS braces stay un-escaped).
const CHAMPIONS_HTML: &str = r##"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>The Hall of Champions // THE SIGNAL</title>
<meta name="description" content="The engine-certified one-of-one champions of THE SIGNAL bloodline, and a live verifier: paste any card code to check whether it is a real, engine-crowned champion. No backend; the engine itself is the notary.">
<meta property="og:title" content="THE SIGNAL // HALL OF CHAMPIONS">
<meta property="og:description" content="Engine-certified one-of-one champions. Verify any card against the source of truth.">
<meta property="og:image" content="__SITE__/og.png">
<meta name="twitter:card" content="summary_large_image">
<link rel="canonical" href="__SITE__/champions.html">
<link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap" rel="stylesheet">
<style>
body{margin:0;background:#0d0f0d;color:#e7e2d4;font-family:'IBM Plex Mono',ui-monospace,monospace}
.s{max-width:980px;margin:0 auto;padding:24px 20px 70px}
.top{display:flex;align-items:center;gap:12px;flex-wrap:wrap}
.b{display:inline-block;background:#e7e2d4;color:#0d0f0d;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:700}
h1{font-size:clamp(26px,6vw,40px);letter-spacing:.04em;margin:14px 0 2px}
.sub{font-size:13px;color:#8d8a7c;line-height:1.6;max-width:680px}
.hd{font-size:11px;letter-spacing:.18em;color:#8d8a7c;margin:26px 0 10px;border-top:1px solid #2a2c28;padding-top:14px}
.vbox{border:1px solid #2a2c28;background:#121411;padding:16px}
.vbox textarea{width:100%;height:60px;background:#0d0f0d;border:1px solid #34362f;color:#cfe7b6;font:inherit;font-size:11px;padding:8px;resize:vertical;box-sizing:border-box}
.vbtn{margin-top:8px;background:none;border:1px solid #4a4d44;color:#e7e2d4;padding:9px 14px;font:inherit;letter-spacing:.08em;font-size:12px;cursor:pointer}
.vbtn:hover{background:#e7e2d4;color:#0d0f0d}
.vres{margin-top:12px;font-size:13px;line-height:1.6;padding:12px 14px;display:none}
.vres.ok{display:block;border:1px solid #2f6f3a;background:#0e160f;color:#bdf0c4}
.vres.warn{display:block;border:1px solid #6a5a14;background:#15120a;color:#ffe39a}
.vres.bad{display:block;border:1px solid #6a2a22;background:#160d0c;color:#ffb4ab}
.vres b{color:#fff}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(240px,1fr));gap:16px}
.cc{border:1px solid #2a2c28;background:#101210;padding:10px}
.cc.cur{border-color:#caa64a;box-shadow:0 0 0 1px #caa64a inset}
.cc canvas{width:100%;height:auto;display:block;border:1px solid #1c1e1a}
.cn{font-size:15px;font-weight:700;margin-top:8px;color:#e7e2d4}
.cn .now{font-size:9px;letter-spacing:.14em;background:#caa64a;color:#15110a;padding:1px 6px;margin-left:6px;vertical-align:middle}
.cm{font-size:11px;color:#cfe7b6;margin-top:2px}
.cs{font-size:11px;color:#a9a596;margin-top:4px}
.cf{font-size:10px;color:#6f6c5f;margin-top:5px;letter-spacing:.04em;word-break:break-all}
.foot{margin-top:30px}
.btn{display:inline-block;border:1px solid #4a4d44;padding:11px 16px;text-decoration:none;color:#e7e2d4;letter-spacing:.06em;font-size:12px;margin:0 8px 8px 0}
.btn:hover{background:#e7e2d4;color:#0d0f0d}
a{color:#cfe7b6}
</style></head>
<body><div class="s">
<div class="top"><span class="b">THE SIGNAL // HALL OF CHAMPIONS</span></div>
<h1>THE HALL OF CHAMPIONS</h1>
<p class="sub">These are the one-of-one champions the engine itself has crowned. There is no login and no server, so the engine is the notary: every day it publishes the authoritative registry of real champions. A card is genuine only if its organism is in that registry. Paste any card code below to check it.</p>
<div class="hd">VERIFY A CARD</div>
<div class="vbox">
<textarea id="vin" placeholder="paste a card code (from a card GIVE or SHARE), or open a verify link..." spellcheck="false" autocomplete="off"></textarea>
<button id="vbtn" type="button" class="vbtn">[ VERIFY ]</button>
<div id="vres" class="vres"></div>
</div>
<div class="hd" id="hallhd">THE CHAMPIONS</div>
<div class="grid" id="hall"><p class="sub">Loading the registry...</p></div>
<div class="foot"><a class="btn" href="__SITE__/bloodline.html">[ BACK TO THE FLOOR ]</a><a class="btn" href="__SITE__/api/certified.json">[ THE REGISTRY, AS JSON ]</a><a class="btn" href="__SITE__/">[ THE SIGNAL ]</a></div>
</div>
<script src="cardart.js"></script>
<script>
var SITE=__SITEJS__;
function esc(t){var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}
function fmt(n){return (n||0).toLocaleString();}
var CERT=[],BYID={},BYNAME={};
function cardFrom(e){return {code:'cert-'+e.id+'-'+e.name,name:e.name,house:e.house,risk:e.risk,finish:'',fade:(e.fade==='FADE'),resolved:{legend:true,podium:true,ascended:((e.best||0)>=1e13||(e.biggest||0)>=1e13),peak:Math.max(e.best||0,e.biggest||0)}};}
function art(cv,card,rar){if(window.CardArt)window.CardArt.drawEmblem(cv,card,rar);}
function renderHall(){var el=document.getElementById('hall'),hd=document.getElementById('hallhd');if(!el)return;
 if(!CERT.length){el.innerHTML='<p class="sub">No champions certified yet. The first is crowned as the bloodline runs.</p>';return;}
 if(hd)hd.textContent='THE CHAMPIONS // '+CERT.length+' CERTIFIED';
 el.innerHTML=CERT.map(function(e,i){return '<div class="cc'+(e.current?' cur':'')+'"><canvas class="cemb" data-i="'+i+'" width="320" height="170"></canvas><div class="cn">'+esc(e.name)+(e.current?' <span class="now">REIGNING</span>':'')+'</div><div class="cm">'+esc(e.house||'')+' // '+(e.fade==='FADE'?'FADE':'TAIL')+'</div><div class="cs">best '+fmt(e.best)+' // W'+(e.max_streak||0)+' // big +'+fmt(e.biggest)+' // '+(e.win_rate||0)+'%</div><div class="cf">CERT '+esc(e.fp)+' // born '+esc(e.born||'')+'</div></div>';}).join('');
 var cv=el.querySelectorAll('canvas.cemb');for(var i=0;i<cv.length;i++){var e=CERT[parseInt(cv[i].getAttribute('data-i'),10)];if(e)art(cv[i],cardFrom(e),'legend');}
}
function decodeCard(str){str=(str||'').trim();if(!str)return null;var o;try{o=JSON.parse(decodeURIComponent(escape(atob(str))));}catch(e){try{o=JSON.parse(atob(str));}catch(e2){o=null;}}if(!o)return null;return o.card||(o.code?o:o);}
function preview(){return '<canvas id="vcanvas" width="320" height="170" style="width:100%;max-width:320px;border:1px solid #2a2c28;margin-top:10px"></canvas>';}
function drawPrev(card,rar){var vc=document.getElementById('vcanvas');if(vc)art(vc,card,rar);}
function verify(str){var el=document.getElementById('vres');if(!el)return;var card=decodeCard(str);
 if(!card||!card.name){el.className='vres bad';el.innerHTML='Not a readable card code. Copy the code from a card GIVE or SHARE and paste the whole thing.';return;}
 var hit=(card.id!=null&&BYID[card.id])||BYNAME[(card.name||'').toUpperCase()];
 if(hit){el.className='vres ok';el.innerHTML='<b>[ CERTIFIED ]</b> '+esc(hit.name)+' is an engine-crowned '+(hit.current?'REIGNING CHAMPION':'champion in the Hall')+'.<br>Engine record: best '+fmt(hit.best)+' chips // streak W'+(hit.max_streak||0)+' // biggest single score +'+fmt(hit.biggest)+' // win rate '+(hit.win_rate||0)+'% // born '+esc(hit.born||'')+'.<br>Certificate <b>'+esc(hit.fp)+'</b>, issued by the engine in CI.'+preview();drawPrev(cardFrom(hit),'legend');}
 else if(card.origin==='pack'||card.origin==='bot'){el.className='vres ok';var rp=(card.rarPack||'card').toUpperCase();el.innerHTML='<b>[ EXOTIC ]</b> <b>'+esc(card.name)+'</b> is a genuine '+esc(rp)+' pack pull'+(card.finish?(' with a '+esc(card.finish.toUpperCase())+' finish'):'')+'. Exotic cards are minted from packs and exist nowhere on the floor; they are real pulls, not engine-crowned floor champions.'+preview();drawPrev(card,(card.resolved&&card.resolved.legend?'legend':(card.resolved&&card.resolved.podium?'rare':'')));}
 else if(card.resolved&&(card.resolved.legend||card.resolved.ascended)){el.className='vres warn';el.innerHTML='<b>[ UNVERIFIED ]</b> This card claims a legendary run, but <b>'+esc(card.name)+'</b> is not in the engine certified Hall of Champions. Any LEGEND or ASCENDED claim on it is unproven.'+preview();drawPrev(card,(card.resolved&&card.resolved.legend?'legend':(card.resolved&&card.resolved.podium?'rare':'')));}
 else{el.className='vres warn';el.innerHTML='<b>[ PLAYER CARD ]</b> <b>'+esc(card.name)+'</b> reads as a genuine player card, but only season champions are engine-certified and this one has not won. Real, just not a champion.'+preview();drawPrev(card,'');}
}
fetch('api/certified.json').then(function(r){return r.json();}).then(function(d){CERT=(d&&d.certified)||[];CERT.forEach(function(e){BYID[e.id]=e;BYNAME[(e.name||'').toUpperCase()]=e;});renderHall();
 var m=/[#&]v=([^&]+)/.exec(location.hash||'');if(m){var code=decodeURIComponent(m[1]);var inp=document.getElementById('vin');if(inp)inp.value=code;verify(code);}
}).catch(function(){var el=document.getElementById('hall');if(el)el.innerHTML='<p class="sub">Could not load the registry.</p>';});
var vb=document.getElementById('vbtn'),vi=document.getElementById('vin');
if(vb)vb.addEventListener('click',function(){verify(vi?vi.value:'');});
</script>
</body></html>"##;

#[allow(clippy::too_many_arguments)]
pub fn render(
    generated_human: &str,
    reveal_delay_days: i64,
    featured_date_human: &str,
    featured: &[Prediction],
    archive: &[Prediction],
    payment_link: &str,
    portal_url: &str,
    early_access_url: &str,
    intake: &serde_json::Value,
    pulse: &serde_json::Value,
    genome: &serde_json::Value,
    engine: &serde_json::Value,
    dreams: &serde_json::Value,
    bloodline: &serde_json::Value,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(crate::OUT_DIR)?;

    // Newest-first. YYYY-MM-DD sorts lexicographically.
    let mut sorted: Vec<&Prediction> = archive.iter().collect();
    sorted.sort_by(|a, b| b.date.cmp(&a.date));
    let total = sorted.len();

    // Group the ledger into dated "pages" (fanfold pages), each with running
    // call numbers (newest = highest).
    let mut pages: Vec<serde_json::Value> = Vec::new();
    let mut i = 0;
    while i < sorted.len() {
        let date = sorted[i].date.clone();
        let mut items = Vec::new();
        while i < sorted.len() && sorted[i].date == date {
            let p = sorted[i];
            let status = if p.status.is_empty() { "OPEN" } else { p.status.as_str() };
            let conf = if p.confidence > 0.0 { p.confidence } else { 0.65 };
            items.push(serde_json::json!({
                "no": total - i,
                "prediction_text": p.prediction_text,
                "source_url": p.source_url,
                "signal_type": p.signal_type,
                "status": status,
                "win_if": p.win_if,
                "resolved_on": p.resolved_on,
                "odds": format!("{:.2}x", 1.0 / conf),
                "conf": (conf * 100.0).round() as i64,
                "market": if p.market.is_empty() { "RESURFACE".to_string() } else { p.market.clone() },
                "rationale": p.rationale,
                "live": p.live,
                "live_delta": p.live - p.live_prev,
                "regime": p.regime,
                "geodesic": p.geodesic,
                "phase": p.phase,
            }));
            i += 1;
        }
        pages.push(serde_json::json!({
            "date": date,
            "human": human_date(&date),
            "count": items.len(),
            "items": items,
        }));
    }

    // A flat oldest-first list for the punch-card "signal map".
    let mut calls: Vec<serde_json::Value> = sorted
        .iter()
        .rev()
        .map(|p| serde_json::json!({ "date": p.date, "signal_type": p.signal_type }))
        .collect();
    if calls.len() > 120 {
        calls = calls.split_off(calls.len() - 120); // keep the most recent 120 dots
    }

    // Record summary: per-source counts across the whole public archive.
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for p in archive {
        *counts.entry(p.signal_type.as_str()).or_insert(0) += 1;
    }
    let mut by_source: Vec<(String, String, usize, i64)> = counts
        .into_iter()
        .map(|(t, c)| {
            let pct = if total > 0 { (c as f64 / total as f64 * 100.0).round() as i64 } else { 0 };
            (t.to_string(), crate::source_label(t).to_string(), c, pct)
        })
        .collect();
    by_source.sort_by(|a, b| b.2.cmp(&a.2));
    let by_source: Vec<serde_json::Value> = by_source
        .into_iter()
        .map(|(t, label, count, pct)| serde_json::json!({ "type": t, "label": label, "count": count, "pct": pct }))
        .collect();

    // The scorecard: the viral artifact. Tally settled and open calls.
    let (mut hits, mut misses, mut open) = (0i64, 0i64, 0i64);
    for p in archive {
        match p.status.as_str() {
            "HIT" => hits += 1,
            "MISS" => misses += 1,
            _ => open += 1,
        }
    }
    let resolved = hits + misses;
    let rate = if resolved > 0 { Some(hits * 100 / resolved) } else { None };
    let verdict = match rate {
        Some(r) if r >= 70 => "THE ORACLE IS BEATING THE STREET",
        Some(r) if r >= 50 => "AHEAD OF THE CROWD",
        Some(_) => "UNDERWATER, AND NOT HIDING IT",
        None => "NO BETS SETTLED YET",
    };
    let scoreboard = serde_json::json!({
        "hits": hits, "misses": misses, "open": open,
        "resolved": resolved, "has_rate": rate.is_some(),
        "rate": rate.unwrap_or(0), "verdict": verdict,
    });

    // THE BOOK: a flat-stake virtual bankroll wagered on the oracle's own
    // calls, settled in chronological order. The line (decimal odds) is 1/conf,
    // so favorites pay little and longshots pay big.
    let mut chrono: Vec<&Prediction> = archive.iter().collect();
    chrono.sort_by(|a, b| a.date.cmp(&b.date));
    let start_bank = 1000.0_f64;
    let stake = 100.0_f64;
    let mut bank = start_bank;
    let mut bank_hist: Vec<f64> = Vec::new();
    let mut bank_dates: Vec<String> = Vec::new();
    let (mut cur, mut best_win, mut best_loss, mut settled) = (0i64, 0i64, 0i64, 0i64);
    let mut last_win: Option<bool> = None;
    for p in &chrono {
        let conf = if p.confidence > 0.0 { p.confidence.clamp(0.34, 0.95) } else { 0.65 };
        let win = match p.status.as_str() {
            "HIT" => true,
            "MISS" => false,
            _ => continue,
        };
        if win {
            bank += stake * ((1.0 / conf) - 1.0);
        } else {
            bank -= stake;
        }
        settled += 1;
        match last_win {
            Some(l) if l == win => cur += 1,
            _ => cur = 1,
        }
        last_win = Some(win);
        if win && cur > best_win { best_win = cur; }
        if !win && cur > best_loss { best_loss = cur; }
        bank_hist.push(bank);
        bank_dates.push(p.date.clone());
    }
    let pnl = bank - start_bank;
    let roi = (pnl / start_bank * 100.0).round() as i64;
    let (mn, mx) = bank_hist.iter().fold((f64::MAX, f64::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    let span = bank_dates.len().saturating_sub(20);
    let book_history: Vec<serde_json::Value> = bank_hist
        .iter()
        .zip(bank_dates.iter())
        .skip(span)
        .map(|(&v, d)| {
            let pct = if mx > mn { ((v - mn) / (mx - mn) * 100.0).round().max(4.0) } else { 50.0 };
            serde_json::json!({ "date": d, "pct": pct as i64 })
        })
        .collect();
    let book = serde_json::json!({
        "bank": bank.round() as i64,
        "roi_str": format!("{}{}%", if pnl >= 0.0 { "+" } else { "" }, roi),
        "pnl_class": if pnl >= 0.0 { "sb-hit" } else { "sb-miss" },
        "streak": match last_win { Some(true) => format!("W{cur}"), Some(false) => format!("L{cur}"), None => "--".to_string() },
        "best_win": best_win, "best_loss": best_loss,
        "settled": settled, "history": book_history,
    });

    // CALIBRATION: the engine grades its own honesty. For every settled call,
    // compare the confidence it set (predicted P(hit)) to the actual outcome.
    // Brier score = mean squared error; lower is better, 0.25 = a coin flip.
    let mut brier_sum = 0.0f64;
    let mut cal_n = 0i64;
    // Three confidence bands: longshots, even money, favorites.
    let bands = [(0.0f64, 0.55f64, "LONGSHOTS"), (0.55, 0.7, "EVEN MONEY"), (0.7, 1.01, "FAVORITES")];
    let mut band_acc: Vec<(f64, i64, i64)> = vec![(0.0, 0, 0); bands.len()]; // sum_pred, hits, n
    for p in archive {
        let outcome = match p.status.as_str() {
            "HIT" => 1.0,
            "MISS" => 0.0,
            _ => continue,
        };
        let conf = if p.confidence > 0.0 { p.confidence.clamp(0.34, 0.95) } else { 0.65 };
        brier_sum += (conf - outcome).powi(2);
        cal_n += 1;
        if let Some(bi) = bands.iter().position(|(lo, hi, _)| conf >= *lo && conf < *hi) {
            band_acc[bi].0 += conf;
            band_acc[bi].1 += outcome as i64;
            band_acc[bi].2 += 1;
        }
    }
    let brier = if cal_n > 0 { (brier_sum / cal_n as f64 * 1000.0).round() / 1000.0 } else { 0.0 };
    let cal_buckets: Vec<serde_json::Value> = bands
        .iter()
        .enumerate()
        .filter(|(i, _)| band_acc[*i].2 > 0)
        .map(|(i, (_, _, label))| {
            let (sum_pred, hits, n) = band_acc[i];
            let pred = (sum_pred / n as f64 * 100.0).round() as i64;
            let actual = (hits * 100) / n;
            serde_json::json!({ "label": label, "pred": pred, "actual": actual, "n": n })
        })
        .collect();
    // Skill grade: how close predicted tracks actual across the bands.
    let cal_err: i64 = cal_buckets
        .iter()
        .map(|b| (b["pred"].as_i64().unwrap_or(0) - b["actual"].as_i64().unwrap_or(0)).abs())
        .sum::<i64>()
        .checked_div(cal_buckets.len().max(1) as i64)
        .unwrap_or(0);
    let cal_grade = match (cal_n, cal_err) {
        (0, _) => "NO SETTLED CALLS YET",
        (_, e) if e <= 8 => "SHARP: THE LINE MEANS WHAT IT SAYS",
        (_, e) if e <= 18 => "HONEST: ROUGHLY CALIBRATED",
        _ => "MISCALIBRATED, AND SHOWING IT",
    };
    let calibration = serde_json::json!({
        "brier": format!("{:.3}", brier),
        "has_data": cal_n > 0,
        "n": cal_n,
        "buckets": cal_buckets,
        "grade": cal_grade,
    });

    let since_human = sorted.last().map(|p| human_date(&p.date)).unwrap_or_default();
    let record = serde_json::json!({
        "total": total,
        "since": since_human,
        "by_source": by_source,
    });

    // Site base URL for feeds, structured data, and share links.
    let site = std::env::var("SITE_URL")
        .unwrap_or_else(|_| "https://mattbusel.github.io/tech-oracle".to_string());
    let site = site.trim_end_matches('/').to_string();
    let ladder_repo = std::env::var("LADDER_REPO")
        .or_else(|_| std::env::var("GITHUB_REPOSITORY"))
        .unwrap_or_else(|_| "Mattbusel/tech-oracle".to_string());

    // JSON-LD structured data (SEO: each call as a CreativeWork in an ItemList).
    let ld_items: Vec<serde_json::Value> = sorted
        .iter()
        .take(15)
        .enumerate()
        .map(|(i, p)| {
            serde_json::json!({
                "@type": "ListItem", "position": i + 1,
                "item": { "@type": "CreativeWork", "headline": p.prediction_text, "datePublished": p.date, "url": format!("{site}/#call-{}", total - i) }
            })
        })
        .collect();
    let jsonld = serde_json::json!({
        "@context": "https://schema.org", "@type": "WebSite", "name": "THE SIGNAL",
        "url": site, "description": "A self-grading public oracle of dated, falsifiable tech predictions.",
        "mainEntity": { "@type": "ItemList", "itemListElement": ld_items }
    })
    .to_string();

    // RSS feed: the syndication source (subscribe, aggregators, IFTTT/Zapier).
    let mut feed = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\"><channel>\n");
    feed.push_str(&format!(
        "<title>THE SIGNAL // dated tech calls</title>\n<link>{site}/</link>\n<description>A self-grading public oracle. Dated, falsifiable tech calls, graded in public.</description>\n"
    ));
    for (i, p) in sorted.iter().enumerate() {
        let no = total - i;
        let title = if p.prediction_text.chars().count() > 90 {
            format!("{}...", p.prediction_text.chars().take(88).collect::<String>())
        } else {
            p.prediction_text.clone()
        };
        let status = if p.status.is_empty() { "OPEN" } else { p.status.as_str() };
        let desc = format!("{} // {} // {}", p.prediction_text, status, p.win_if);
        feed.push_str(&format!(
            "<item><title>{}</title><link>{}/#call-{}</link><guid isPermaLink=\"false\">signal-{}</guid><pubDate>{}</pubDate><description>{}</description></item>\n",
            xml(&title), site, no, no, rfc822(&p.date), xml(&desc)
        ));
    }
    feed.push_str("</channel></rss>\n");
    std::fs::write(format!("{}/feed.xml", crate::OUT_DIR), feed)?;

    // Programmatic SEO: one crawlable permalink page per revealed call.
    let _ = std::fs::create_dir_all(format!("{}/call", crate::OUT_DIR));
    let mut urls = vec![format!("{site}/")];
    let mut img_entries = vec![format!(
        "<url><loc>{site}/</loc><image:image><image:loc>{site}/og.png</image:loc><image:title>THE SIGNAL: daily tech oracle and live betting pit</image:title></image:image></url>"
    )];
    for (i, p) in sorted.iter().enumerate() {
        let no = total - i;
        let status = if p.status.is_empty() { "OPEN" } else { p.status.as_str() };
        let market = if p.market.is_empty() { "RESURFACE" } else { p.market.as_str() };
        let desc = xml(&clip_r(&p.prediction_text, 150));
        let tt = xml(&clip_r(&p.prediction_text, 65));
        // The call as a dated, machine-readable claim. A resolved call becomes a
        // ClaimReview (the date proves we called it first); an open call is a
        // dated Claim. This is what lets search and AI engines cite the receipt.
        let claim_text = clip_r(&p.prediction_text, 240);
        let claim_ld = match p.status.as_str() {
            "HIT" | "MISS" => {
                let (rv, name) = if p.status == "HIT" { (5, "Resolved HIT: the call was correct") } else { (1, "Resolved MISS: the call was wrong") };
                serde_json::json!({
                    "@context": "https://schema.org", "@type": "ClaimReview",
                    "datePublished": p.date, "url": format!("{site}/call/{no}.html"),
                    "claimReviewed": claim_text,
                    "author": { "@type": "Organization", "name": "THE SIGNAL", "url": site },
                    "reviewRating": { "@type": "Rating", "ratingValue": rv, "bestRating": 5, "worstRating": 1, "alternateName": name },
                    "itemReviewed": { "@type": "Claim", "datePublished": p.date, "author": { "@type": "Organization", "name": "THE SIGNAL" }, "appearance": { "@type": "CreativeWork", "url": format!("{site}/call/{no}.html") } }
                })
            }
            _ => serde_json::json!({
                "@context": "https://schema.org", "@type": "CreativeWork",
                "headline": claim_text, "datePublished": p.date,
                "url": format!("{site}/call/{no}.html"),
                "author": { "@type": "Organization", "name": "THE SIGNAL", "url": site }
            }),
        }.to_string();
        let page = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>Call No. {no}: {tt} // THE SIGNAL</title>\n<meta name=\"description\" content=\"{desc}\">\n<meta property=\"og:title\" content=\"THE SIGNAL // Call No. {no} [{status}]\">\n<meta property=\"og:description\" content=\"{desc}\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n<meta property=\"og:image\" content=\"{site}/call/{no}.png\">\n<meta name=\"twitter:image\" content=\"{site}/call/{no}.png\">\n<link rel=\"canonical\" href=\"{site}/call/{no}.html\">\n<script type=\"application/ld+json\">{ld}</script>\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:620px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}.c{{font-size:25px;font-weight:600;line-height:1.35;margin:18px 0}}.m{{font-size:11px;letter-spacing:.1em;color:#6d6b5e}}.w{{font-size:12px;color:#6d6b5e;margin:14px 0}}.r{{display:inline-block;font-weight:600;padding:3px 10px;letter-spacing:.12em;font-size:12px}}.r-hit{{background:#1f7a3d;color:#efede4}}.r-miss{{background:#b23a2e;color:#efede4}}a{{color:#1b1a14}}</style></head>\n<body><div class=\"s\"><div class=\"b\">THE SIGNAL // CALL No. {no}</div>\n<div class=\"m\">{date} // {market} // {status}</div>\n{receipt}<p class=\"c\">{t}</p>\n<div class=\"w\">{win}</div>\n<p class=\"m\"><a href=\"{src}\" rel=\"noopener\">source signal</a> // <a href=\"{site}/receipts.html\">the receipts</a> // <a href=\"{site}/#call-{no}\">on the public record</a> // <a href=\"{site}/\">THE SIGNAL</a></p>\n</div></body></html>\n",
            no = no, tt = tt, t = xml(&p.prediction_text), desc = desc, status = status, market = market,
            date = xml(&p.date), win = xml(&p.win_if), src = xml(&p.source_url), site = site, ld = claim_ld,
            receipt = match p.status.as_str() {
                "HIT" => format!("<p><span class=\"r r-hit\">CALLED IT // HIT</span> <span class=\"m\">called {} // resolved {} // {} days on the record</span></p>\n", p.date, p.resolved_on, day_diff(&p.date, &p.resolved_on)),
                "MISS" => format!("<p><span class=\"r r-miss\">ON THE RECORD // MISS</span> <span class=\"m\">called {} // resolved {} // no edits, no deletes</span></p>\n", p.date, p.resolved_on),
                _ => String::new(),
            }
        );
        let _ = std::fs::write(format!("{}/call/{no}.html", crate::OUT_DIR), page);
        urls.push(format!("{site}/call/{no}.html"));
        // og:image for this call page
        let _ = crate::card::call_card(
            &format!("{}/call/{no}.png", crate::OUT_DIR),
            &site, no as i64, status, market, &p.prediction_text,
        );
        img_entries.push(format!(
            "<url><loc>{site}/call/{no}.html</loc><image:image><image:loc>{site}/call/{no}.png</image:loc><image:caption>{cap}</image:caption></image:image></url>",
            cap = xml(&clip_r(&p.prediction_text, 140))
        ));
    }
    // Topic pages: group the archive by subject so the site matches real search
    // queries ("<topic> predictions"), not just one call's exact wording.
    let mut topics: BTreeMap<String, Vec<(usize, &Prediction)>> = BTreeMap::new();
    for (i, p) in sorted.iter().enumerate() {
        if !p.keyword.is_empty() {
            topics.entry(slug(&p.keyword)).or_default().push((total - i, p));
        }
    }
    let _ = std::fs::create_dir_all(format!("{}/topic", crate::OUT_DIR));
    for (sl, calls) in &topics {
        let topic = sl.to_uppercase();
        let items: String = calls
            .iter()
            .map(|(no, p)| {
                let st = if p.status.is_empty() { "OPEN" } else { p.status.as_str() };
                format!(
                    "<li class=\"i\"><span class=\"m\">{date} // {st}</span><br><a href=\"{site}/call/{no}.html\">{t}</a></li>",
                    date = xml(&p.date), st = st, no = no, t = xml(&clip_r(&p.prediction_text, 130)), site = site
                )
            })
            .collect();
        let page = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{topic}: dated tech predictions // THE SIGNAL</title>\n<meta name=\"description\" content=\"Every dated, self-graded call on {topic} from THE SIGNAL, a public tech-prediction oracle.\">\n<meta property=\"og:title\" content=\"THE SIGNAL // {topic} predictions\">\n<meta property=\"og:image\" content=\"{site}/og.png\">\n<link rel=\"canonical\" href=\"{site}/topic/{sl}.html\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:640px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}h1{{font-size:26px;letter-spacing:.04em}}ul{{list-style:none;padding:0}}.i{{padding:12px 0;border-bottom:1px dashed rgba(27,26,20,.3);font-size:15px;line-height:1.4}}.m{{font-size:11px;letter-spacing:.1em;color:#6d6b5e}}a{{color:#1b1a14}}</style></head>\n<body><div class=\"s\"><div class=\"b\">THE SIGNAL // TOPIC</div>\n<h1>{topic}</h1><p class=\"m\">Dated, self-graded calls on {topic}.</p>\n<ul>{items}</ul>\n<p class=\"m\"><a href=\"{site}/\">THE SIGNAL // the full record</a></p></div></body></html>\n",
            topic = topic, sl = sl, items = items, site = site
        );
        let _ = std::fs::write(format!("{}/topic/{sl}.html", crate::OUT_DIR), page);
        urls.push(format!("{site}/topic/{sl}.html"));
    }

    // THE RECEIPTS: the credibility wall. Every dated call that has settled,
    // newest first, with how many days early it went on the record. "We called
    // it, here is the proof" is the most shareable thing the engine produces.
    {
        let mut hit_rows = String::new();
        let mut miss_rows = String::new();
        let (mut nh, mut nm) = (0i64, 0i64);
        for (i, p) in sorted.iter().enumerate() {
            let no = total - i;
            match p.status.as_str() {
                "HIT" => {
                    nh += 1;
                    hit_rows.push_str(&format!(
                        "<li class=\"i\"><div class=\"rh\"><span class=\"r r-hit\">HIT</span> <span class=\"lead\">{lead} DAYS ON THE RECORD</span></div><a href=\"{site}/call/{no}.html\">{t}</a><div class=\"meta\">CALLED {called} // RESOLVED {res} // {mk}</div></li>",
                        lead = day_diff(&p.date, &p.resolved_on), site = site, no = no,
                        t = xml(&clip_r(&p.prediction_text, 150)), called = xml(&p.date), res = xml(&p.resolved_on),
                        mk = if p.market.is_empty() { "RESURFACE" } else { p.market.as_str() }
                    ));
                }
                "MISS" => {
                    nm += 1;
                    miss_rows.push_str(&format!(
                        "<li class=\"i\"><div class=\"rh\"><span class=\"r r-miss\">MISS</span> <span class=\"lead\">NO EDITS, NO DELETES</span></div><a href=\"{site}/call/{no}.html\">{t}</a><div class=\"meta\">CALLED {called} // RESOLVED {res} // {mk}</div></li>",
                        site = site, no = no, t = xml(&clip_r(&p.prediction_text, 150)), called = xml(&p.date), res = xml(&p.resolved_on),
                        mk = if p.market.is_empty() { "RESURFACE" } else { p.market.as_str() }
                    ));
                }
                _ => {}
            }
        }
        let hits_block = if nh > 0 { format!("<h2>CALLED IT // HITS</h2><ul>{hit_rows}</ul>") } else { String::new() };
        let miss_block = if nm > 0 { format!("<h2>ON THE RECORD // MISSES</h2><ul>{miss_rows}</ul>") } else { String::new() };
        let body = if nh + nm > 0 {
            format!("{hits_block}{miss_block}")
        } else {
            "<p class=\"empty\">No calls have settled yet. The first receipts print soon. Nothing here is editable once it does.</p>".to_string()
        };
        let receipts = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>The Receipts: tech predictions THE SIGNAL called first, dated and graded</title>\n<meta name=\"description\" content=\"Every dated, self-graded tech prediction from THE SIGNAL, printed before it resolved. {nh} hits and {nm} misses on the public record. No edits, no deletes.\">\n<meta property=\"og:title\" content=\"THE SIGNAL // THE RECEIPTS [{nh}-{nm}]\">\n<meta property=\"og:description\" content=\"We called it, here is the dated proof. {nh} hits, {nm} misses, every one on the record.\">\n<meta property=\"og:image\" content=\"{site}/og.png\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n<link rel=\"canonical\" href=\"{site}/receipts.html\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:680px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}h1{{font-size:30px;letter-spacing:.04em;margin:18px 0 4px}}h2{{font-size:13px;letter-spacing:.16em;border-top:1px solid rgba(27,26,20,.25);padding-top:14px;margin-top:26px}}.sub{{font-size:13px;color:#6d6b5e;line-height:1.5}}ul{{list-style:none;padding:0}}.i{{padding:13px 0;border-bottom:1px dashed rgba(27,26,20,.3)}}.i a{{color:#1b1a14;font-size:15px;font-weight:600;line-height:1.4;text-decoration:none}}.i a:hover{{text-decoration:underline}}.rh{{margin-bottom:5px}}.r{{display:inline-block;font-weight:600;padding:2px 9px;letter-spacing:.12em;font-size:11px}}.r-hit{{background:#1f7a3d;color:#efede4}}.r-miss{{background:#b23a2e;color:#efede4}}.lead{{font-size:10.5px;letter-spacing:.1em;color:#6d6b5e;margin-left:6px}}.meta{{font-size:10.5px;letter-spacing:.08em;color:#6d6b5e;margin-top:5px}}.empty{{color:#6d6b5e}}a{{color:#1b1a14}}</style></head>\n<body><div class=\"s\"><div class=\"b\">THE SIGNAL // THE RECEIPTS</div>\n<h1>WE CALLED IT</h1>\n<p class=\"sub\">Every prediction below was printed and dated before it resolved. The machine grades itself in public: {nh} hits, {nm} misses on the record. No edits, no deletes, only prints.</p>\n{body}\n<p class=\"meta\"><a href=\"{site}/\">back to THE SIGNAL</a> // <a href=\"{site}/dataset/\">the open dataset</a></p></div></body></html>\n",
            nh = nh, nm = nm, site = site, body = body
        );
        std::fs::write(format!("{}/receipts.html", crate::OUT_DIR), receipts)?;
        urls.push(format!("{site}/receipts.html"));
    }

    // THE ARENA: a serverless prediction tournament. Anyone or any AI agent
    // enters a dated bet by opening a GitHub issue labeled "arena" with a
    // SIGNAL-BET line. The board is rendered client-side: it reads the issues
    // and the public record, settles every bet, and ranks players (with the
    // engine itself and an anti-oracle as standing competitors). GitHub Issues
    // is the database; there is no server.
    {
        let arena = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>The Arena: humans and AI agents vs the machine // THE SIGNAL</title>\n<meta name=\"description\" content=\"A serverless prediction tournament. Tail or fade the oracle's dated calls; the board settles every bet against the public record and ranks every player, human or AI, against the machine.\">\n<meta property=\"og:title\" content=\"THE SIGNAL // THE ARENA\">\n<meta property=\"og:description\" content=\"Humans and AI agents bet against the machine. Every call dated, every bet settled in public.\">\n<meta property=\"og:image\" content=\"{site}/og.png\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n<link rel=\"canonical\" href=\"{site}/arena.html\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:720px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}h1{{font-size:30px;letter-spacing:.04em;margin:18px 0 4px}}.sub{{font-size:13px;color:#6d6b5e;line-height:1.55}}table{{width:100%;border-collapse:collapse;margin-top:14px}}th,td{{text-align:left;padding:9px 8px;border-bottom:1px dashed rgba(27,26,20,.3);font-size:13px}}th{{font-size:10.5px;letter-spacing:.12em;color:#6d6b5e}}.rank{{color:#6d6b5e;width:28px}}.you{{background:rgba(91,240,138,.18)}}.eng{{font-weight:700}}.sc-up{{color:#1f7a3d;font-weight:700}}.sc-dn{{color:#b23a2e;font-weight:700}}.title{{font-size:10.5px;letter-spacing:.08em;color:#6d6b5e}}.btn{{display:inline-block;text-align:center;border:1.5px solid #1b1a14;padding:12px 18px;margin:14px 8px 0 0;text-decoration:none;color:#1b1a14;font-weight:600;letter-spacing:.06em;cursor:pointer;background:none}}.btn:hover{{background:#1b1a14;color:#efede4}}code{{background:rgba(27,26,20,.1);padding:1px 5px}}.fmt{{font-size:12px;color:#3b3a30;background:rgba(27,26,20,.06);padding:12px;margin-top:14px;line-height:1.6;white-space:pre-wrap}}a{{color:#1b1a14}}</style></head>\n<body><div class=\"s\"><div class=\"b\">THE SIGNAL // THE ARENA</div>\n<h1>BEAT THE MACHINE</h1>\n<p class=\"sub\">Tail or fade the oracle's dated calls. Every bet is settled in public against the record, no edits, no deletes. The machine and its shadow, the anti-oracle, stand on the board as permanent competitors. Humans enter from their browser; AI agents enter through the API. There is no server: the entries are public GitHub issues.</p>\n<div id=\"board\"><p class=\"sub\">Reading the record and settling the floor...</p></div>\n<a class=\"btn\" id=\"enter\">[ ENTER A BET ]</a>\n<a class=\"btn\" href=\"{site}/\">[ BACK TO THE SIGNAL ]</a>\n<div class=\"fmt\">HOW TO ENTER<br>Humans: tap ENTER A BET (it opens a prefilled GitHub issue).<br>Agents: open an issue on the repo, label it <code>arena</code>, body containing one line:<br>  SIGNAL-BET kw=&lt;keyword&gt; market=&lt;MARKET&gt; side=&lt;TAIL|FADE&gt; by=&lt;your handle&gt;<br>TAIL backs the machine's call; FADE bets against it. Settled HIT/MISS from {site}/api/record.json. Bet on calls listed in {site}/api/today.json.</div>\n<script>\nvar REPO={repo}, SITE={site_js};\nvar TITLES=[[10,'LEGEND OF THE DEN'],[6,'ORACLE-KILLER'],[3,'SHARP'],[1,'CONTENDER'],[-2,'ROOKIE'],[-1e9,'THE MARK']];\nfunction titleFor(s){{for(var i=0;i<TITLES.length;i++)if(s>=TITLES[i][0])return TITLES[i][1];return 'ROOKIE';}}\nfunction esc(t){{var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}}\nfunction mine(){{try{{var c=localStorage.getItem('signal_cred');return c?c.split('-')[0].toUpperCase():null;}}catch(e){{return null;}}}}\nvar BET=/SIGNAL-BET\\s+kw=(\\S+)\\s+market=(\\S+)\\s+side=(TAIL|FADE)\\s+by=(.+)/i;\nPromise.all([\n  fetch('api/record.json').then(function(r){{return r.ok?r.json():null;}}).catch(function(){{return null;}}),\n  fetch('https://api.github.com/repos/'+REPO+'/issues?labels=arena&state=all&per_page=100',{{headers:{{Accept:'application/vnd.github+json'}}}}).then(function(r){{return r.ok?r.json():[];}}).catch(function(){{return [];}})\n]).then(function(res){{\n  var rec=res[0]||{{}}, issues=res[1]||[];\n  var calls=(rec.calls)||[]; var byKw={{}};\n  calls.forEach(function(c){{var k=(c.keyword||'').toLowerCase();if(k&&!byKw[k])byKw[k]=c;}});\n  var players={{}};\n  function P(by){{if(!players[by])players[by]={{by:by,w:0,l:0,p:0}};return players[by];}}\n  issues.forEach(function(it){{var m=BET.exec(it.body||'');if(!m)return;var kw=m[1].toLowerCase(),side=m[3].toUpperCase(),by=(m[4]||'').trim().slice(0,24).toUpperCase()||'ANON';var c=byKw[kw];var pl=P(by);if(!c||c.status==='OPEN'){{pl.p++;return;}}var win=(c.status==='HIT')===(side==='TAIL');if(win)pl.w++;else pl.l++;}});\n  var sb=rec.scoreboard||{{hits:0,misses:0}};\n  var rows=Object.keys(players).map(function(k){{var p=players[k];p.score=p.w-p.l;return p;}});\n  rows.push({{by:'THE MACHINE',w:sb.hits||0,l:sb.misses||0,p:sb.open||0,score:(sb.hits||0)-(sb.misses||0),eng:1}});\n  rows.push({{by:'THE ANTI-ORACLE',w:sb.misses||0,l:sb.hits||0,p:0,score:(sb.misses||0)-(sb.hits||0),eng:1}});\n  rows.sort(function(a,b){{return b.score-a.score;}});\n  var me=mine();\n  var html='<table><tr><th class=rank>#</th><th>PLAYER</th><th>W-L</th><th>SCORE</th><th>TITLE</th></tr>';\n  rows.forEach(function(p,i){{var cls=(p.eng?'eng':'')+((me&&p.by===me)?' you':'');var sc=(p.score>=0?'+':'')+p.score;html+='<tr class=\"'+cls+'\"><td class=rank>'+(i+1)+'</td><td>'+esc(p.by)+'</td><td>'+p.w+'-'+p.l+(p.p?(' ('+p.p+'open)'):'')+'</td><td class=\"'+(p.score>=0?'sc-up':'sc-dn')+'\">'+sc+'</td><td class=title>'+titleFor(p.score)+'</td></tr>';}});\n  html+='</table>';\n  if(!issues.length)html+='<p class=\"sub\">No challengers yet. The machine is undefeated by default. Be the first to enter.</p>';\n  document.getElementById('board').innerHTML=html;\n}});\ndocument.getElementById('enter').addEventListener('click',function(e){{e.preventDefault();var by=mine()||'anon';var calls=[];try{{}}catch(e2){{}}var kw=prompt('Keyword to bet on (see today.json on the site):','');if(!kw)return;var side=(prompt('TAIL (back the machine) or FADE (bet against it)?','TAIL')||'TAIL').toUpperCase();if(side!=='TAIL'&&side!=='FADE')side='TAIL';var body='My bet in THE SIGNAL arena.\\n\\nSIGNAL-BET kw='+kw.toLowerCase().replace(/[^a-z0-9]/g,'')+' market=ANY side='+side+' by='+by+'\\n\\nThe board: '+SITE+'arena.html';var url='https://github.com/'+REPO+'/issues/new?labels=arena&title='+encodeURIComponent('Arena: '+by+' '+side+' '+kw)+'&body='+encodeURIComponent(body);window.open(url,'_blank','noopener');}});\n</script>\n</div></body></html>\n",
            site = site, repo = serde_json::to_string(&ladder_repo).unwrap_or_else(|_| "\"\"".to_string()),
            site_js = serde_json::to_string(&format!("{site}/")).unwrap_or_else(|_| "\"\"".to_string())
        );
        std::fs::write(format!("{}/arena.html", crate::OUT_DIR), arena)?;
        urls.push(format!("{site}/arena.html"));
    }

    // SLEEP MODE: a destination, not a takeover. A living dreamscape that never
    // stops, recombining the corpus into new far-future calls forever, client
    // side. Reached on purpose; it never ambushes anyone.
    {
        let pool_js = serde_json::to_string(dreams.get("pool").unwrap_or(&serde_json::json!([]))).unwrap_or_else(|_| "[]".to_string());
        let forms_js = serde_json::to_string(dreams.get("forms").unwrap_or(&serde_json::json!([]))).unwrap_or_else(|_| "[]".to_string());
        let sleep = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>Sleep Mode: the oracle dreams // THE SIGNAL</title>\n<meta name=\"description\" content=\"The oracle never stops. In sleep mode it recombines everything it has seen into surreal far-future calls, endlessly. A living dreamscape.\">\n<meta property=\"og:title\" content=\"THE SIGNAL // SLEEP MODE\">\n<meta property=\"og:description\" content=\"The oracle dreams while you are away. A living, always-running dreamscape.\">\n<meta property=\"og:image\" content=\"{site}/og.png\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n<link rel=\"canonical\" href=\"{site}/sleep.html\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>html,body{{margin:0;height:100%}}body{{background:#07060f;color:#c3b4ff;font-family:'IBM Plex Mono',ui-monospace,monospace;overflow:hidden}}#field{{position:fixed;inset:0;transition:background 8s linear;background:radial-gradient(circle at 50% 30%,#171247,#07060f 72%)}}.wrap{{position:relative;z-index:2;height:100%;display:flex;flex-direction:column}}.top{{padding:26px 24px 8px;text-align:center;flex:0 0 auto}}.h{{font-size:12px;letter-spacing:.42em;color:#8a7ad6}}.t{{font-size:11px;letter-spacing:.18em;color:#6a5db0;margin-top:8px}}.count{{font-size:10.5px;letter-spacing:.16em;color:#4e4488;margin-top:6px}}#stream{{flex:1 1 auto;overflow:hidden;position:relative;padding:10px 24px 30px}}.dream{{max-width:660px;margin:0 auto;font-size:clamp(16px,3.6vw,24px);line-height:1.5;text-align:center;padding:14px 0;opacity:0;transform:translateY(14px);transition:opacity 2.2s ease,transform 2.2s ease}}.dream.in{{opacity:.92;transform:none}}.dream.out{{opacity:0;transform:translateY(-22px)}}.dream b{{color:#e7deff;font-weight:700}}.deep{{font-size:clamp(22px,5vw,34px);color:#efe9ff}}.bar{{position:fixed;bottom:0;left:0;right:0;z-index:3;display:flex;gap:16px;justify-content:center;padding:14px;background:linear-gradient(transparent,#07060f 60%)}}.bar a,.bar button{{color:#c3b4ff;background:none;border:1px solid rgba(195,180,255,.35);padding:9px 16px;text-decoration:none;font-family:inherit;letter-spacing:.1em;font-size:12px;cursor:pointer}}.bar a:hover,.bar button:hover{{background:rgba(195,180,255,.12)}}.star{{position:fixed;width:2px;height:2px;background:#b9a7ff;border-radius:50%;opacity:0;z-index:1;animation:tw 6s ease-in-out infinite}}@keyframes tw{{0%,100%{{opacity:0}}50%{{opacity:.5}}}}@media (prefers-reduced-motion:reduce){{.dream{{transition:none}}.star{{animation:none}}}}</style></head>\n<body>\n<div id=\"field\"></div>\n<div class=\"wrap\"><div class=\"top\"><div class=\"h\">THE ORACLE IS DREAMING</div><div class=\"t\">IT RECOMBINES WHAT IT HAS SEEN INTO FUTURES THAT DO NOT EXIST YET</div><div class=\"count\" id=\"count\">DREAM No. 0</div></div><div id=\"stream\"></div></div>\n<div class=\"bar\"><a href=\"{site}/\">[ WAKE THE ORACLE ]</a><button id=\"faster\" type=\"button\">[ DREAM FASTER ]</button></div>\n<script>\nvar POOL={pool_js}, FORMS={forms_js};\nif(!POOL.length){{POOL=['THE SIGNAL','THE MACHINE','THE FUTURE'];}}\nif(!FORMS.length){{FORMS=['In the long night, {{a}} and {{b}} are the same machine.'];}}\nvar stream=document.getElementById('stream'),countEl=document.getElementById('count'),field=document.getElementById('field');\nvar n=0, speed=3400, hues=[245,265,225,285,205];\nfunction pick(a){{return a[Math.floor(Math.random()*a.length)];}}\nfunction esc(t){{var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}}\nfunction make(){{var a=pick(POOL),b=pick(POOL);for(var i=0;i<4&&b===a;i++)b=pick(POOL);var f=pick(FORMS);return f.split('{{a}}').join('<b>'+esc(a)+'</b>').split('{{b}}').join('<b>'+esc(b)+'</b>');}}\nfunction emit(){{n++;countEl.textContent='DREAM No. '+n;var el=document.createElement('div');var deep=Math.random()<0.15;el.className='dream'+(deep?' deep':'');el.innerHTML=make();stream.appendChild(el);requestAnimationFrame(function(){{el.classList.add('in');}});var kids=stream.children;if(kids.length>9){{var old=kids[0];old.classList.add('out');setTimeout(function(){{if(old.parentNode)old.parentNode.removeChild(old);}},2300);}}var hue=pick(hues);field.style.background='radial-gradient(circle at '+(30+Math.random()*40)+'% '+(20+Math.random()*30)+'%, hsl('+hue+',55%,18%), #07060f 72%)';}}\nfunction loop(){{emit();setTimeout(loop, speed + (Math.random()*1600-400));}}\nfor(var s=0;s<40;s++){{var st=document.createElement('div');st.className='star';st.style.left=(Math.random()*100)+'vw';st.style.top=(Math.random()*100)+'vh';st.style.animationDelay=(Math.random()*6)+'s';document.body.appendChild(st);}}\nemit();emit();setTimeout(loop,speed);\ndocument.getElementById('faster').addEventListener('click',function(){{speed=speed>1400?1500:3400;this.textContent=speed<2000?'[ DREAM SLOWER ]':'[ DREAM FASTER ]';}});\n</script>\n</body></html>\n",
            site = site, pool_js = pool_js, forms_js = forms_js
        );
        std::fs::write(format!("{}/sleep.html", crate::OUT_DIR), sleep)?;
        urls.push(format!("{site}/sleep.html"));
    }

    // THE MANIFOLD: the prediction core made visible. A live plot of every topic
    // as a point on the relativistic attention manifold (regime, conviction,
    // geodesic forecast), plus THE PROVING GROUND: the head-to-head benchmark of
    // the manifold against the canonical algorithms. Reads api/observatory.json
    // and api/benchmark.json at runtime, so it is always current.
    {
        let tpl = r###"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>The Manifold: the prediction core // THE SIGNAL</title>
<meta name="description" content="The oracle predicts through a manifold. Every topic is a trajectory through a curved relativistic space-time; its regime and geodesic forecast drive the calls. Benchmarked head-to-head against momentum, PageRank, and the rest.">
<meta property="og:title" content="THE SIGNAL // THE MANIFOLD">
<meta property="og:description" content="The prediction core: topics as geodesics on a curved manifold, benchmarked against the algorithms that run the internet.">
<meta property="og:image" content="__SITE__/og.png">
<meta name="twitter:card" content="summary_large_image">
<link rel="canonical" href="__SITE__/manifold.html">
<link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap" rel="stylesheet">
<style>
html,body{margin:0;background:#06060c;color:#cfe9ff;font-family:'IBM Plex Mono',ui-monospace,monospace}
.s{max-width:760px;margin:0 auto;padding:40px 26px 80px}
.b{display:inline-block;background:#cfe9ff;color:#06060c;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:700}
h1{font-size:32px;letter-spacing:.04em;margin:16px 0 4px}
h2{font-size:13px;letter-spacing:.22em;color:#6f86a8;margin:38px 0 10px;border-bottom:1px solid rgba(111,134,168,.3);padding-bottom:8px}
.sub{font-size:13px;color:#8aa0bf;line-height:1.6}
canvas{width:100%;height:420px;display:block;background:radial-gradient(circle at 50% 42%,#0e1330,#06060c 74%);border:1px solid rgba(111,134,168,.25);margin-top:14px}
.legend{display:flex;gap:18px;flex-wrap:wrap;font-size:11px;letter-spacing:.08em;margin-top:10px;color:#8aa0bf}
.dot{display:inline-block;width:9px;height:9px;border-radius:50%;margin-right:5px;vertical-align:middle}
table{width:100%;border-collapse:collapse;margin-top:12px}
th,td{text-align:left;padding:8px 7px;border-bottom:1px dashed rgba(111,134,168,.28);font-size:13px}
th{font-size:10px;letter-spacing:.12em;color:#6f86a8}
td.num{text-align:right;font-variant-numeric:tabular-nums}
tr.me{background:rgba(95,200,255,.12)}
tr.me td{color:#dff1ff;font-weight:700}
.win{color:#5fd08a;font-weight:700}
.heat td{text-align:center;font-size:11.5px;color:#06060c;font-weight:600}
.heat th{text-align:center}
.note{font-size:12px;color:#6f86a8;background:rgba(111,134,168,.08);padding:12px;margin-top:14px;line-height:1.6}
.btn{display:inline-block;border:1.5px solid #cfe9ff;padding:11px 17px;margin:22px 8px 0 0;text-decoration:none;color:#cfe9ff;font-weight:600;letter-spacing:.06em}
.btn:hover{background:#cfe9ff;color:#06060c}
code{background:rgba(111,134,168,.16);padding:1px 5px}
a{color:#cfe9ff}
</style></head>
<body><div class="s">
<div class="b">THE SIGNAL // THE MANIFOLD</div>
<h1>THE PREDICTION CORE</h1>
<p class="sub">The oracle does not predict on a flat line. Every topic is a trajectory through a curved relativistic space-time, and where it goes next is a geodesic on that manifold. Repurposed from a quant engine built to predict asset prices through topology, the core reads each topic's relativistic velocity, its Lorentz factor (fast moves carry more weight), its space-time regime, and a forward geodesic forecast. That regime shapes which bet the machine makes; the geodesic predicts the line.</p>

<h2>THE ORACLE BOX</h2>
<p class="sub">Type any tracked topic. The algorithm runs in your browser and forecasts it on the spot: its regime, its phase, the day it turns, and the probability it rises. No server, no wait.</p>
<div style="display:flex;gap:8px;margin-top:10px;flex-wrap:wrap">
<input id="obox" placeholder="rust, claude, nvidia, ai, llm, apple..." autocomplete="off" spellcheck="false" style="flex:1;min-width:200px;background:#0e1330;border:1px solid rgba(111,134,168,.4);color:#cfe9ff;font-family:inherit;font-size:14px;padding:11px 12px;letter-spacing:.04em">
<button id="obtn" class="btn" style="margin:0">[ FORECAST ]</button>
</div>
<div id="obres" class="note" style="display:none"></div>
<canvas id="ospark" width="708" height="92" style="width:100%;height:92px;display:none;background:#0e1330;border:1px solid rgba(111,134,168,.25);margin-top:10px"></canvas>

<h2>YOUR WATCHLIST</h2>
<p class="sub">Track topics and the manifold watches them for you, live. A peak or trough forming lights up as an alert. Saved in your browser, recomputed continuously.</p>
<div id="watch"></div>

<h2>THE LIVE MANIFOLD</h2>
<p class="sub">Each point is a topic the engine is tracking, placed by its forward geodesic forecast (left falls, right rises) and its conviction (higher sits stronger). Color is the local regime.</p>
<canvas id="m" width="708" height="420"></canvas>
<div class="legend">
<span><span class="dot" style="background:#5fd08a"></span>TIMELIKE / causal trend</span>
<span><span class="dot" style="background:#5fb8ff"></span>LIGHTLIKE / transition</span>
<span><span class="dot" style="background:#e88a4d"></span>SPACELIKE / stochastic</span>
</div>
<div id="mnote" class="note">Reading the manifold...</div>

<h2>THE PROVING GROUND</h2>
<p class="sub" id="benchhead">Benchmarking the manifold against the algorithms that run the internet...</p>
<table id="bench"></table>
<p class="sub" style="margin-top:18px">Directional accuracy by regime, the hardest test. Trend-followers ace clean trends and collapse on reversals; the manifold holds up across all of them.</p>
<div style="overflow-x:auto"><table id="heat" class="heat"></table></div>
<div class="note" id="realnote"></div>

<a class="btn" href="__SITE__/">[ BACK TO THE SIGNAL ]</a>
<a class="btn" href="__SITE__/api/benchmark.json">[ THE BENCHMARK, AS JSON ]</a>

<script>
var REG={TIMELIKE:'#5fd08a',LIGHTLIKE:'#5fb8ff',SPACELIKE:'#e88a4d'};
function esc(t){var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}
var cv=document.getElementById('m'),ctx=cv.getContext('2d'),DATA=[],tphase=0;
function resize(){var r=cv.getBoundingClientRect();cv.width=r.width;cv.height=420;}
window.addEventListener('resize',resize);resize();
function draw(){
 var W=cv.width,H=cv.height;ctx.clearRect(0,0,W,H);
 // curved manifold mesh
 ctx.strokeStyle='rgba(111,134,168,.16)';ctx.lineWidth=1;
 for(var gy=0;gy<=8;gy++){ctx.beginPath();for(var gx=0;gx<=W;gx+=12){var yy=H*gy/8+Math.sin((gx/W*6)+(gy*0.6)+tphase*0.4)*8;if(gx===0)ctx.moveTo(gx,yy);else ctx.lineTo(gx,yy);}ctx.stroke();}
 for(var gx2=0;gx2<=10;gx2++){var xx=W*gx2/10;ctx.beginPath();ctx.moveTo(xx,0);ctx.lineTo(xx,H);ctx.globalAlpha=.5;ctx.stroke();ctx.globalAlpha=1;}
 // center axis (zero forecast) and light cone
 ctx.strokeStyle='rgba(111,134,168,.4)';ctx.beginPath();ctx.moveTo(W/2,0);ctx.lineTo(W/2,H);ctx.stroke();
 ctx.fillStyle='#6f86a8';ctx.font='10px IBM Plex Mono';ctx.fillText('FALLS',12,H-12);ctx.fillText('RISES',W-46,H-12);ctx.fillText('CONVICTION',12,18);
 var pts=DATA.filter(function(d){return d.defined;});
 if(!pts.length){ctx.fillStyle='#8aa0bf';ctx.font='13px IBM Plex Mono';ctx.textAlign='center';ctx.fillText('THE MANIFOLD IS WARMING UP',W/2,H/2-6);ctx.font='11px IBM Plex Mono';ctx.fillText('topics need a few days of history before a geodesic is defined',W/2,H/2+14);ctx.textAlign='left';return;}
 pts.forEach(function(d,i){
  // Live drift: each point wanders around its geodesic forecast every frame, so
  // the plot is alive without a reload (mean-reverting to the forecast + jitter).
  if(d.live==null)d.live=d.trend;
  d.live += (d.trend - d.live)*0.01 + (Math.random()-0.5)*0.02;
  if(d.live>1)d.live=1; else if(d.live<-1)d.live=-1;
  var x=W/2 + d.live*(W/2-50);
  var conv=Math.max(0,Math.min(1,1-1/Math.max(1,d.gamma)));
  var y=H-30 - conv*(H-70) + Math.sin(tphase+i)*3;
  var rad=4+conv*9, col=REG[d.regime]||'#8aa0bf';
  // geodesic arrow
  ctx.strokeStyle=col;ctx.lineWidth=2;ctx.beginPath();ctx.moveTo(x,y);ctx.lineTo(x+d.trend*34,y);ctx.stroke();
  ctx.fillStyle=col;ctx.beginPath();ctx.arc(x,y,rad,0,7);ctx.fill();
  ctx.fillStyle='rgba(207,233,255,.85)';ctx.font='10px IBM Plex Mono';ctx.fillText(d.term,x+rad+3,y-4);
 });
}
function loop(){tphase+=0.03;draw();requestAnimationFrame(loop);}
fetch('api/observatory.json').then(function(r){return r.json();}).then(function(o){
 var m=(o&&o.manifold)||[];
 DATA=m.map(function(d){return {term:d.term,regime:d.regime,gamma:d.gamma,trend:d.geodesic_trend,defined:d.defined};});
 var def=DATA.filter(function(d){return d.defined;}).length;
 document.getElementById('mnote').innerHTML=def?(def+' topics have a defined trajectory. Points drift live as new days land.'):'No topic has 3+ days of history yet, so every reading is warming up. This plot fills in as the corpus matures, one daily run at a time. The math is live now; the picture follows.';
}).catch(function(){document.getElementById('mnote').textContent='Could not load the live manifold.';});
loop();

fetch('api/benchmark.json').then(function(r){return r.json();}).then(function(d){
 var b=d&&d.benchmark;if(!b){return;}
 var t=document.getElementById('bench');
 t.innerHTML='<tr><th>ALGORITHM</th><th>WHAT IT IS</th><th class=num>ACC</th><th class=num>IC</th><th class=num>BRIER</th></tr>';
 b.algos.forEach(function(a){
  var tr=document.createElement('tr');if(a.is_manifold)tr.className='me';
  tr.innerHTML='<td>'+esc(a.name)+'</td><td class=sub style="font-size:11px">'+esc(a.blurb)+'</td><td class=num>'+a.accuracy.toFixed(1)+'%</td><td class=num>'+a.ic.toFixed(3)+'</td><td class=num>'+a.brier.toFixed(3)+'</td>';
  t.appendChild(tr);
 });
 var bits=[];if(b.manifold_best_ic)bits.push('the highest information coefficient');if(b.manifold_best_brier)bits.push('the best calibration (Brier)');
 var head='Across '+b.samples.toLocaleString()+' forecasts over '+b.regimes.length+' market regimes, THE MANIFOLD posts '+(bits.length?bits.join(' and '):'a top-tier score')+' of the '+b.algos.length+' algorithms tested. ';
 head+='IC is the standard quant yardstick (correlation of the forecast with what actually happened); Brier measures calibration, lower is better. The field includes EWMA momentum (the engine behind hot-feed ranking), moving-average crossover (classic technical analysis), PageRank (Google’s importance-by-structure), a recommender popularity baseline, and the random-walk null.';
 document.getElementById('benchhead').innerHTML=head;
 // heatmap
 var h=document.getElementById('heat');
 var hh='<tr><th>ALGORITHM</th>'+b.regimes.map(function(r){return '<th>'+esc(r)+'</th>';}).join('')+'</tr>';
 b.algos.forEach(function(a){
  var byr={};a.by_regime.forEach(function(x){byr[x.regime]=x.acc;});
  hh+='<tr'+(a.is_manifold?' class=me':'')+'><td style="text-align:left;color:#cfe9ff;font-weight:600">'+esc(a.name)+'</td>'+b.regimes.map(function(r){
   var v=byr[r]||0;var g=Math.max(0,Math.min(1,(v-30)/65));var bg='rgba('+Math.round(232-g*170)+','+Math.round(138+g*70)+','+Math.round(77+g*10)+','+(0.25+g*0.65)+')';
   return '<td style="background:'+bg+'">'+v.toFixed(0)+'%</td>';
  }).join('')+'</tr>';
 });
 h.innerHTML=hh;
 var rn=document.getElementById('realnote');
 rn.innerHTML=b.real_eligible>0?(b.real_eligible+' live topics now have enough history to be graded for real; the live benchmark is activating.'):'This is a controlled benchmark on synthetic topics with known regimes, designed to be fair (momentum should tie on clean trends; everyone should land at a coin flip on noise). A live benchmark on the real corpus activates automatically once topics accumulate 30+ days of history.';
}).catch(function(){document.getElementById('benchhead').textContent='Could not load the benchmark.';});
</script>
<script src="manifold.js"></script>
<script>
(function(){
 var TRAJ=null, WKEY='signal_watch_v1';
 var PHCOL={RISING:'#6ee07a',PEAKING:'#e88a4d',FALLING:'#ff8c7d',BOTTOMING:'#5fb8ff',CHURNING:'#8aa0bf',FLAT:'#6f86a8'};
 function esc(t){var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}
 function loadW(){try{return JSON.parse(localStorage.getItem(WKEY)||'[]');}catch(e){return [];}}
 function saveW(a){try{localStorage.setItem(WKEY,JSON.stringify(a));}catch(e){}}
 fetch('api/trajectories.json').then(function(r){return r.json();}).then(function(d){TRAJ=(d&&d.series)||{};drawWatch();}).catch(function(){});
 function read(term){
  if(!TRAJ||!window.Manifold)return null;
  var key=(term||'').trim().toUpperCase(), ser=TRAJ[key];
  if(!ser){var ks=Object.keys(TRAJ).filter(function(k){return k.indexOf(key)===0;});if(ks.length){key=ks[0];ser=TRAJ[key];}}
  if(!ser)return null;
  var r=window.Manifold.analyze(ser); r.term=key; r.series=ser; return r;
 }
 function spark(r){
  var c=document.getElementById('ospark');if(!c)return;c.style.display='block';
  var ctx=c.getContext('2d'),W=c.width,H=c.height;ctx.clearRect(0,0,W,H);
  var s=r.series.slice(-60),n=s.length,fut=r.path?r.path(7):[];
  var lastLog=Math.log(1+(s[n-1]||0)),proj=fut.map(function(f){return Math.exp(lastLog+f)-1;});
  var lo=Math.min.apply(null,s.concat(proj)),hi=Math.max.apply(null,s.concat(proj).concat([1]));
  function Y(v){return H-6-(v-lo)/((hi-lo)||1)*(H-12);}
  ctx.strokeStyle='#6f86a8';ctx.lineWidth=1.5;ctx.beginPath();
  for(var i=0;i<n;i++){var x=i/(n+6)*W;if(i===0)ctx.moveTo(x,Y(s[i]));else ctx.lineTo(x,Y(s[i]));}
  ctx.stroke();
  ctx.strokeStyle=PHCOL[r.phase]||'#cfe9ff';ctx.setLineDash([4,3]);ctx.beginPath();ctx.moveTo((n-1)/(n+6)*W,Y(s[n-1]));
  for(var j=0;j<proj.length;j++)ctx.lineTo((n+j)/(n+6)*W,Y(proj[j]));
  ctx.stroke();ctx.setLineDash([]);
 }
 function showForecast(term){
  var el=document.getElementById('obres');if(!el)return;el.style.display='block';
  var r=read(term);
  if(!r){el.innerHTML='No trajectory for "'+esc(term)+'" yet. Try a tracked topic, like rust, claude, nvidia, ai, apple, llm.';document.getElementById('ospark').style.display='none';return;}
  var col=PHCOL[r.phase]||'#cfe9ff',turn=r.peakIn?(' // turns in ~'+r.peakIn+'d'):'';
  el.innerHTML='<b style="color:#dff1ff;font-size:15px">'+esc(r.term)+'</b> &mdash; <b style="color:'+col+'">'+r.phase+'</b> ('+r.regime+')'+turn+'<br>P(rise) <b>'+Math.round(r.prob*100)+'%</b> // geodesic '+(r.trend>=0?'+':'')+Math.round(r.trend*100)+'% // gamma '+r.gamma.toFixed(2)+'<br><button id="obwadd" class="btn" style="margin:9px 0 0;padding:6px 10px;font-size:11px">[ + ADD TO WATCHLIST ]</button>';
  spark(r);
  var ab=document.getElementById('obwadd');if(ab)ab.onclick=function(){var w=loadW();if(w.indexOf(r.term)<0){w.push(r.term);saveW(w);drawWatch();}this.textContent='[ WATCHING ]';};
 }
 var ob=document.getElementById('obox'),obtn=document.getElementById('obtn');
 if(obtn)obtn.addEventListener('click',function(){showForecast(ob.value||'');});
 if(ob)ob.addEventListener('keydown',function(e){if(e.key==='Enter')showForecast(ob.value||'');});
 function drawWatch(){
  var el=document.getElementById('watch');if(!el)return;var w=loadW();
  if(!w.length){el.innerHTML='<p class="sub">Watchlist empty. Forecast a topic above, then add it.</p>';return;}
  el.innerHTML=w.map(function(term){
   var r=read(term),col=r?(PHCOL[r.phase]||'#cfe9ff'):'#6f86a8';
   var line=r?('<b style="color:'+col+'">'+r.phase+'</b>'+(r.peakIn?(' ~'+r.peakIn+'d'):'')+' // P(rise) '+Math.round(r.prob*100)+'%'):'no data';
   var al=(r&&(r.phase==='PEAKING'||r.phase==='BOTTOMING'))?' <span style="color:'+col+';font-weight:700">* ALERT *</span>':'';
   return '<div style="display:flex;justify-content:space-between;align-items:center;gap:8px;padding:8px 0;border-bottom:1px dashed rgba(111,134,168,.28)"><span class="nm" style="overflow:hidden;text-overflow:ellipsis">'+esc(term)+' &mdash; '+line+al+'</span><button data-rm="'+esc(term)+'" class="btn" style="margin:0;padding:5px 9px;font-size:11px;flex:0 0 auto">[ x ]</button></div>';
  }).join('');
  var bs=el.querySelectorAll('[data-rm]');for(var i=0;i<bs.length;i++)bs[i].addEventListener('click',function(){var t=this.getAttribute('data-rm');saveW(loadW().filter(function(x){return x!==t;}));drawWatch();});
 }
 setInterval(drawWatch,5000);
})();
</script>
</div></body></html>
"###;
        let manifold_page = tpl.replace("__SITE__", site.as_str());
        std::fs::write(format!("{}/manifold.html", crate::OUT_DIR), manifold_page)?;
        urls.push(format!("{site}/manifold.html"));
    }

    // THE EVENT HORIZON: the reversals the manifold is calling. A momentum feed can
    // only tell you what is already hot; this names what is about to turn, peaking
    // or bottoming, with a projected day. The manifold's signature edge, productized.
    {
        let tpl = r###"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>The Event Horizon: what is about to turn // THE SIGNAL</title>
<meta name="description" content="A live board of the reversals the machine is calling: topics peaking or bottoming, with the projected day of the turn. Momentum tells you what is already hot. The manifold tells you what is about to flip.">
<meta property="og:title" content="THE SIGNAL // THE EVENT HORIZON">
<meta property="og:description" content="The reversals the machine is calling before they happen. Peaks and troughs, dated.">
<meta property="og:image" content="__SITE__/og.png">
<meta name="twitter:card" content="summary_large_image">
<link rel="canonical" href="__SITE__/horizon.html">
<link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap" rel="stylesheet">
<style>
html,body{margin:0;background:#0a0710;color:#f3e6d0;font-family:'IBM Plex Mono',ui-monospace,monospace}
.s{max-width:760px;margin:0 auto;padding:40px 26px 80px}
.b{display:inline-block;background:#f3e6d0;color:#0a0710;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:700}
h1{font-size:32px;letter-spacing:.04em;margin:16px 0 4px}
h2{font-size:13px;letter-spacing:.22em;color:#a98b6f;margin:36px 0 8px;border-bottom:1px solid rgba(169,139,111,.3);padding-bottom:8px}
.sub{font-size:13px;color:#c2a98c;line-height:1.6}
.turn{display:flex;align-items:center;gap:14px;padding:14px 12px;border-bottom:1px dashed rgba(169,139,111,.3)}
.tg{font-size:11px;font-weight:700;letter-spacing:.12em;padding:4px 9px;border-radius:2px;white-space:nowrap}
.peak{background:rgba(232,138,77,.2);color:#f0a86a;border:1px solid rgba(232,138,77,.5)}
.bottom{background:rgba(95,200,255,.16);color:#6fc7ff;border:1px solid rgba(95,200,255,.45)}
.tm{font-size:17px;font-weight:700;color:#fff7ea;flex:1}
.tw{font-size:11px;color:#a98b6f;text-align:right;line-height:1.5}
.big{font-size:21px;color:#f0a86a;font-weight:700}
.phases{display:flex;gap:10px;flex-wrap:wrap;margin-top:12px}
.ph{font-size:11px;letter-spacing:.08em;padding:6px 11px;border:1px solid rgba(169,139,111,.35);color:#c2a98c}
.ph b{color:#fff7ea}
.note{font-size:12px;color:#a98b6f;background:rgba(169,139,111,.08);padding:12px;margin-top:16px;line-height:1.6}
.btn{display:inline-block;border:1.5px solid #f3e6d0;padding:11px 17px;margin:24px 8px 0 0;text-decoration:none;color:#f3e6d0;font-weight:600;letter-spacing:.06em}
.btn:hover{background:#f3e6d0;color:#0a0710}
a{color:#f3e6d0}
</style></head>
<body><div class="s">
<div class="b">THE SIGNAL // THE EVENT HORIZON</div>
<h1>WHAT IS ABOUT TO TURN</h1>
<p class="sub">A momentum feed can only tell you what is already hot, which is why it buys every top and sells every bottom. This board is the opposite: the topics the manifold reads as <b>turning</b> right now, rising into a peak or falling into a trough, each with the projected day of the turn. In the benchmark this is exactly where the manifold pulls ahead of the trend-followers, so this is the page where it earns its keep.</p>

<h2>THE TURNS</h2>
<div id="turns"><p class="sub">Scanning the manifold for reversals...</p></div>

<h2>THE FIELD</h2>
<p class="sub">Every tracked topic, sorted into its phase. The shape of the whole discourse at a glance.</p>
<div class="phases" id="phases"></div>
<div class="note" id="note"></div>

<a class="btn" href="__SITE__/manifold.html">[ THE MANIFOLD ]</a>
<a class="btn" href="__SITE__/">[ BACK TO THE SIGNAL ]</a>

<script>
function esc(t){var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}
fetch('api/horizon.json').then(function(r){return r.json();}).then(function(d){
 var h=d&&d.horizon;if(!h){return;}
 var box=document.getElementById('turns');
 if(!h.turns||!h.turns.length){
  box.innerHTML='<p class="sub">No turns called yet. Either the field is trending cleanly, or topics are still building the history the manifold needs (it reads a trajectory after a few days). This board fills in as the corpus matures, and a called peak that lands becomes a receipt no momentum feed could have printed.</p>';
 } else {
  box.innerHTML=h.turns.map(function(t){
   var peak=t.phase==='PEAKING';
   var when=t.peak_in>0?('in ~'+t.peak_in+'d'):'imminent';
   return '<div class="turn"><span class="tg '+(peak?'peak':'bottom')+'">'+(peak?'PEAK':'TROUGH')+'</span>'
    +'<span class="tm">'+esc(t.term)+'</span>'
    +'<span class="tw"><span class="big">'+when+'</span><br>peaked '+(t.peak||0)+' // seen '+(t.active_days||0)+'d // P(rise) '+t.prob_rising+'%</span></div>';
  }).join('');
 }
 var ph=h.phases||{};var order=['RISING','PEAKING','FALLING','BOTTOMING','CHURNING','FLAT'];
 var pe=document.getElementById('phases');
 var any=Object.keys(ph).length;
 pe.innerHTML=any?order.filter(function(k){return ph[k];}).map(function(k){return '<span class="ph"><b>'+ph[k]+'</b> '+k+'</span>';}).join(''):'<span class="sub">No topic has enough history to phase yet.</span>';
 document.getElementById('note').innerHTML='Scanned '+(h.scanned||0)+' tracked topics; '+(h.defined||0)+' have enough trajectory to read. A peak means the topic is rising now but its geodesic has already curved down; a trough is the mirror. The reversal is what the others cannot see.';
}).catch(function(){document.getElementById('turns').textContent='Could not load the event horizon.';});
</script>
</div></body></html>
"###;
        let horizon_page = tpl.replace("__SITE__", site.as_str());
        std::fs::write(format!("{}/horizon.html", crate::OUT_DIR), horizon_page)?;
        urls.push(format!("{site}/horizon.html"));
    }

    // THE BLOODLINE, LIVE: a broadcast you tune into. The day's population is
    // baked in; the client runs it as a live channel with animated standings, a
    // house race, an events feed, and a rolling commentary you can switch the
    // voice on for. Always running.
    {
        let bl_js = serde_json::to_string(bloodline).unwrap_or_else(|_| "{}".to_string());
        let gen = bloodline.get("gen").and_then(|v| v.as_i64()).unwrap_or(0);

        // Collectible cards: PRO (top career stat lines), ROOKIE (promising young),
        // HALL OF FAME (all-time greats). Real dot-matrix PNGs per organism.
        let _ = std::fs::create_dir_all(format!("{}/bloodline/cards", crate::OUT_DIR));
        // Cards are namespaced by kind (pro-/rookie-/hof-), so the same organism
        // can hold both a Pro card and a Hall of Fame card.
        let card_for = |o: &serde_json::Value, kind: &str, slug: &str| {
            let id = o.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            let s = |k: &str| o.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let i = |k: &str| o.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
            let f = |k: &str| o.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let roi = i("roi");
            let stats = vec![
                ("ROI", format!("{}{}%", if roi >= 0 { "+" } else { "" }, roi)),
                ("WIN RATE", format!("{}%", i("win_rate"))),
                ("BEST STREAK", format!("W{}", i("max_streak"))),
                ("BIGGEST WIN", format!("+{}", i("biggest"))),
                ("BIGGEST BET", i("big_bet").to_string()),
                ("LIFESPAN", format!("{}d", i("age"))),
            ];
            let aggr_pct = (((f("aggr") + 0.20) / 0.40) * 100.0) as i64;
            let genes = vec![("AGGR", aggr_pct), ("RISK", (f("risk") * 100.0) as i64), ("SELECT", i("select")), ("PRESS", i("press"))];
            let best_s = i("best").to_string();
            let _ = crate::card::organism_card(
                &format!("{}/bloodline/cards/{}-{}.png", crate::OUT_DIR, slug, id),
                kind, &s("name"), &s("house"), &s("born"), ("BEST", &best_s),
                &stats, &genes, &s("fade"), &site,
            );
        };
        let empty: Vec<serde_json::Value> = Vec::new();
        let card_set = |arr_key: &str, kind: &str, slug: &str, n: usize| {
            for o in bloodline.get(arr_key).and_then(|v| v.as_array()).unwrap_or(&empty).iter().take(n) {
                card_for(o, kind, slug);
            }
        };
        card_set("pros", "PRO CARD", "pro", 3);
        card_set("rookies", "ROOKIE CARD", "rookie", 3);
        card_set("hall_of_fame", "HALL OF FAME", "hof", 3);
        let bl_page = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>The Bloodline, Live: the oracle is a breeding species // THE SIGNAL</title>\n<meta name=\"description\" content=\"Watch the oracle evolve. A live broadcast of a breeding population of gambler-organisms: standings, rival houses, births and deaths, with a champion that takes the line. Natural selection you can tune into.\">\n<meta property=\"og:title\" content=\"THE SIGNAL // THE BLOODLINE, LIVE\">\n<meta property=\"og:description\" content=\"A living population of strategies that breed, mutate and die by their bets. Tune in. Generation {gen}.\">\n<meta property=\"og:image\" content=\"{site}/og.png\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n<link rel=\"canonical\" href=\"{site}/bloodline.html\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#0d0f0d;color:#e7e2d4;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:860px;margin:0 auto;padding:24px 20px 60px}}.top{{display:flex;align-items:center;gap:12px;flex-wrap:wrap}}.b{{display:inline-block;background:#e7e2d4;color:#0d0f0d;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:700}}.air{{display:inline-flex;align-items:center;gap:7px;color:#ff5a4d;font-size:11px;letter-spacing:.2em;font-weight:700}}.air .dot{{width:9px;height:9px;border-radius:50%;background:#ff5a4d;animation:pulse 1.4s infinite}}@keyframes pulse{{0%,100%{{opacity:.3}}50%{{opacity:1}}}}h1{{font-size:clamp(26px,6vw,40px);letter-spacing:.04em;margin:14px 0 2px}}.sub{{font-size:12px;color:#8d8a7c;line-height:1.5;max-width:620px}}.comm{{margin:18px 0;border:1px solid #2a2c28;background:#121411;padding:16px 16px;min-height:58px;display:flex;align-items:center;gap:14px}}.comm .txt{{font-size:clamp(15px,3.2vw,20px);line-height:1.4;flex:1}}.comm b{{color:#ffd56b}}.listen{{flex:0 0 auto;background:none;border:1px solid #4a4d44;color:#e7e2d4;padding:9px 12px;font-family:inherit;letter-spacing:.08em;font-size:11px;cursor:pointer}}.listen.on{{background:#ff5a4d;border-color:#ff5a4d;color:#0d0f0d}}.grid{{display:grid;grid-template-columns:1.4fr 1fr;gap:22px;margin-top:8px}}@media(max-width:680px){{.grid{{grid-template-columns:1fr}}}}.hd{{font-size:11px;letter-spacing:.18em;color:#8d8a7c;margin:18px 0 8px;border-top:1px solid #2a2c28;padding-top:12px}}.row{{margin:7px 0}}.rt{{display:flex;justify-content:space-between;font-size:13px;gap:8px}}.rt .nm{{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}.rt .ft{{color:#ffd56b;font-weight:700;flex:0 0 auto}}.bar{{height:6px;background:#1c1e1a;margin-top:4px;overflow:hidden}}.bar i{{display:block;height:100%;background:#6ee07a;width:0;transition:width 1.1s cubic-bezier(.4,0,.2,1)}}.row.champ .nm{{color:#ffd56b;font-weight:700}}.row.champ .bar i{{background:#ffd56b}}.hse .rt .nm{{color:#cfe7b6}}.hse .bar i{{background:#9ac46a}}.evt{{font-size:12px;color:#a9a596;padding:6px 0;border-bottom:1px dotted #2a2c28}}.evt b{{color:#e7e2d4}}.evt.die b{{color:#ff8c7d}}.evt.born b{{color:#7fe0a0}}.tag{{font-size:9px;letter-spacing:.12em;color:#6f6c5f;border:1px solid #34362f;padding:1px 5px;margin-left:6px}}.foot{{margin-top:26px}}.btn{{display:inline-block;border:1px solid #4a4d44;padding:11px 16px;text-decoration:none;color:#e7e2d4;letter-spacing:.06em;font-size:12px}}.btn:hover{{background:#e7e2d4;color:#0d0f0d}}a{{color:#cfe7b6}}.cards{{display:flex;gap:12px;flex-wrap:wrap;margin-top:6px}}.cards figure{{margin:0;width:148px}}.cards img{{width:148px;display:block;border:1px solid #2a2c28}}.cards figcaption{{font-size:10px;color:#8d8a7c;margin-top:4px;letter-spacing:.05em}}.cards img{{cursor:zoom-in;transition:transform .15s ease}}.cards img:hover{{transform:translateY(-3px)}}.hof .rt .nm{{color:#ffd56b}}.hof .bar i{{background:#caa64a}}#cardzoom{{display:none;position:fixed;inset:0;z-index:50;background:rgba(7,8,7,.93);align-items:center;justify-content:center;cursor:zoom-out;padding:18px;flex-direction:column;gap:12px}}#cardzoom.on{{display:flex}}#cardzoom img{{max-width:92vw;max-height:84vh;border:1px solid #2a2c28;box-shadow:0 0 50px rgba(0,0,0,.7)}}#cardzoom .x{{color:#8d8a7c;font-size:11px;letter-spacing:.18em}}.bigmoney{{margin:10px 0 0;border:1px solid #4a3a12;background:#15110a;color:#ffd56b;padding:12px 14px;font-size:clamp(13px,2.8vw,18px);font-weight:700;letter-spacing:.03em;display:flex;align-items:center;gap:10px;min-height:22px}}.bigmoney .tag{{background:#ffd56b;color:#15110a;font-size:10px;padding:2px 7px;letter-spacing:.16em;flex:0 0 auto}}.bigmoney.flash{{animation:shove 1.1s cubic-bezier(.2,.8,.2,1)}}@keyframes shove{{0%{{transform:scale(.93);background:#3a2e08}}35%{{transform:scale(1.02);background:#4a3a0c}}100%{{transform:scale(1);background:#15110a}}}}@media(prefers-reduced-motion:reduce){{.bigmoney.flash{{animation:none}}}}</style></head>\n<body><div class=\"s\">\n<div class=\"top\"><span class=\"b\">THE SIGNAL // THE BLOODLINE</span><span class=\"air\"><span class=\"dot\"></span>ON AIR</span></div>\n<h1>THE BLOODLINE, LIVE</h1>\n<p class=\"sub\">The oracle is a breeding population. Every organism shadow-bets the whole record with its own inherited nerve; the rich survive and mate, the broke die, and the champion sets tomorrow's real line. This is the channel. It does not stop.</p>\n<div class=\"comm\"><div class=\"txt\" id=\"comm\">tuning in...</div><button class=\"listen\" id=\"listen\" type=\"button\">[ LISTEN ]</button></div>\n<div class=\"bigmoney\" id=\"bigmoney\"></div>\n<div class=\"grid\"><div><div class=\"hd\" id=\"tablehd\">THE TABLE // LIVING, BY SHADOW BANKROLL</div><div id=\"table\"></div></div><div><div class=\"hd\">THE HOUSES</div><div id=\"houses\"></div><div class=\"hd\">THE WIRE // BIRTHS &amp; DEATHS</div><div id=\"events\"></div></div></div>\n<div class=\"hd\">THE CARDS // PROS &amp; ROOKIES</div><div class=\"cards\" id=\"cards\"></div>\n<div class=\"hd\">HALL OF FAME // ALL-TIME GREATS</div><div id=\"hof\"></div>\n<div class=\"foot\"><a class=\"btn\" href=\"{site}/\">[ BACK TO THE SIGNAL ]</a></div>\n</div>\n<div id=\"cardzoom\"></div>\n<script>\nvar BL={bl_js};var MKT_REPO={repo_js};\nfunction esc(t){{var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}}\nvar living=(BL.living||[]), houses=(BL.houses||[]), dead=(BL.dead||[]), newborns=(BL.newborns||[]);\nvar maxfit=Math.max.apply(null,(living.length?living:[{{fitness:1}}]).map(function(o){{return o.fitness||1;}}));\nvar tablehd=document.getElementById('tablehd');\nif(tablehd)tablehd.textContent='THE TABLE // GEN '+(BL.gen||0)+' // '+(living.length)+' ALIVE OF '+(BL.total_ever||living.length)+' EVER';\nfunction drawTable(){{var el=document.getElementById('table');if(!el)return;el.innerHTML=living.map(function(o,i){{var jit=1+(Math.random()*0.04-0.02);var shown=Math.round((o.fitness||0)*jit);var w=Math.max(3,Math.round((o.fitness||0)/maxfit*100));return '<div class=\"row'+(i===0?' champ':'')+'\"><div class=rt><span class=nm>'+(i+1)+'. '+esc(o.name)+(i===0?' (CHAMPION)':'')+'<span class=tag>'+esc(o.house||'')+'</span></span><span class=ft>'+shown+'</span></div><div class=bar><i style=\"width:'+w+'%\"></i></div><div class=g style=\"margin-top:3px\">'+(o.wins||0)+'-'+(o.losses||0)+' on '+(o.bets||0)+' bets ('+(o.win_rate||0)+'%) // W'+(o.max_streak||0)+' // ROI '+((o.roi>=0?'+':'')+(o.roi||0))+'% // '+esc(o.fade||'TAIL')+'</div></div>';}}).join('')||'<p class=sub>The founding generation is being born.</p>';}}\nfunction drawHouses(){{var el=document.getElementById('houses');if(!el)return;var mh=Math.max.apply(null,(houses.length?houses:[{{fitness:1}}]).map(function(h){{return h.fitness||1;}}));el.innerHTML=houses.map(function(h){{var w=Math.max(4,Math.round((h.fitness||0)/mh*100));return '<div class=\"row hse\"><div class=rt><span class=nm>'+esc(h.name)+' ('+h.count+')</span><span class=ft>'+h.fitness+'</span></div><div class=bar><i style=\"width:'+w+'%\"></i></div></div>';}}).join('')||'<p class=sub>no houses yet</p>';}}\nfunction drawEvents(){{var el=document.getElementById('events');if(!el)return;var ev=[];newborns.forEach(function(o){{ev.push('<div class=\"evt born\">BORN <b>'+esc(o.name)+'</b> // '+esc(o.house||'')+'</div>');}});dead.forEach(function(o){{ev.push('<div class=\"evt die\">FALLEN <b>'+esc(o.name)+'</b> // lived '+(o.age||0)+'d, final '+(o.fitness||0)+'</div>');}});el.innerHTML=ev.join('')||'<p class=sub>quiet round. no births, no deaths.</p>';}}\nvar champ=living[0], runner=living[1];\nvar LINES=[];\nLINES.push('Generation <b>'+(BL.gen||0)+'</b> is live. <b>'+living.length+'</b> organisms at the table.');\nif(champ)LINES.push('<b>'+esc(champ.name)+'</b> holds the line with <b>'+champ.fitness+'</b> chips. The house bets through it now.');\nif(champ&&runner)LINES.push('<b>'+esc(runner.name)+'</b> is closing, only <b>'+Math.max(0,(champ.fitness-runner.fitness))+'</b> behind the champion.');\nif(houses[0])LINES.push('<b>'+esc(houses[0].name)+'</b> lead the houses with <b>'+houses[0].fitness+'</b> across '+houses[0].count+' members.');\nif(houses.length>1)LINES.push('<b>'+esc(houses[houses.length-1].name)+'</b> are thinning out. Temperament is destiny here.');\nnewborns.slice(0,2).forEach(function(o){{LINES.push('Fresh blood: <b>'+esc(o.name)+'</b> just sat down, untested, betting like '+esc(o.house||'a stranger')+'.');}});\ndead.slice(0,2).forEach(function(o){{LINES.push('<b>'+esc(o.name)+'</b> busted out after '+(o.age||0)+' days. The bloodline does not mourn.');}});\nif(champ)LINES.push('The champion '+esc(champ.name)+' runs aggr '+champ.aggr+', risk '+champ.risk+'. Bold enough to live, for now.');\nif(champ&&champ.win_rate)LINES.push('<b>'+esc(champ.name)+'</b> is hitting <b>'+champ.win_rate+'%</b> with a best run of W'+(champ.max_streak||0)+' and a biggest single score of +'+(champ.biggest||0)+'.');\nif(BL.hall_of_fame&&BL.hall_of_fame[0])LINES.push('All-time, no organism has beaten <b>'+esc(BL.hall_of_fame[0].name)+'</b> and its career high of <b>'+(BL.hall_of_fame[0].best||0)+'</b> chips.');\nif(!LINES.length)LINES.push('The table is being set. Check back as the species is born.');\nvar ci=0, listening=false, commEl=document.getElementById('comm'), btn=document.getElementById('listen');\nfunction speak(t){{if(!('speechSynthesis'in window))return;try{{speechSynthesis.cancel();var u=new SpeechSynthesisUtterance(t.replace(/<[^>]+>/g,''));u.rate=.92;u.pitch=.8;speechSynthesis.speak(u);}}catch(e){{}}}}\nfunction nextLine(){{var l=LINES[ci%LINES.length];ci++;if(commEl){{commEl.style.opacity=0;setTimeout(function(){{commEl.innerHTML=l;commEl.style.opacity=1;}},250);}}if(listening)speak(l);}}\ncommEl.style.transition='opacity .25s ease';\nbtn.addEventListener('click',function(){{listening=!listening;btn.classList.toggle('on',listening);btn.textContent=listening?'[ ON AIR ]':'[ LISTEN ]';if(listening)speak(LINES[(ci-1+LINES.length)%LINES.length]);else if('speechSynthesis'in window)speechSynthesis.cancel();}});\nfunction drawCards(){{var el=document.getElementById('cards');if(!el)return;var list=[];(BL.pros||[]).slice(0,3).forEach(function(o){{list.push([o,'pro','PRO']);}});(BL.rookies||[]).slice(0,3).forEach(function(o){{list.push([o,'rookie','ROOKIE']);}});if(!list.length){{el.innerHTML='<p class=sub>cards print as the season runs.</p>';return;}}el.innerHTML=list.map(function(p){{var o=p[0];return '<figure><img loading=lazy alt=\"'+esc(o.name)+'\" src=\"bloodline/cards/'+p[1]+'-'+o.id+'.png\"><figcaption>'+p[2]+' // '+esc(o.name)+'</figcaption></figure>';}}).join('');}}\nfunction drawHof(){{var el=document.getElementById('hof');if(!el)return;var h=(BL.hall_of_fame||[]),mx=((h[0]&&h[0].best)||1);el.innerHTML=h.map(function(o,i){{var w=Math.max(4,Math.round((o.best||0)/mx*100));return '<div class=\"row hof\"><div class=rt><span class=nm>'+(i+1)+'. '+esc(o.name)+'<span class=tag>'+esc(o.house||'')+'</span></span><span class=ft>'+(o.best||0)+'</span></div><div class=bar><i style=\"width:'+w+'%\"></i></div></div>';}}).join('')||'<p class=sub>no legends yet. the first is inducted soon.</p>';}}\ndrawTable();drawHouses();drawEvents();drawCards();drawHof();nextLine();\nvar bigEl=document.getElementById('bigmoney');\nfunction fmt(n){{return (n||0).toLocaleString();}}\nvar SHOVES=(living||[]).filter(function(o){{return (o.big_bet||0)>250||(o.biggest||0)>250;}}).sort(function(a,b){{return (b.biggest||0)-(a.biggest||0);}});\nfunction shoveLine(o){{var verb=(o.fade==='FADE')?'FADING THE MACHINE':'BACKING THE CALL';return '<span class=tag>BIG MONEY</span><span>'+esc(o.name)+' SHOVED '+fmt(o.big_bet)+' '+verb+' AND TOOK DOWN +'+fmt(o.biggest)+' ON A SINGLE BET</span>';}}\nvar svi=0;\nfunction nextShove(){{if(!bigEl)return;if(!SHOVES.length){{bigEl.style.display='none';return;}}var o=SHOVES[svi%SHOVES.length];svi++;bigEl.innerHTML=shoveLine(o);bigEl.classList.remove('flash');void bigEl.offsetWidth;bigEl.classList.add('flash');}}\nnextShove();setInterval(nextShove,5800);\nvar zoom=document.getElementById('cardzoom');\ndocument.getElementById('cards').addEventListener('click',function(e){{var im=e.target.closest('img');if(!im)return;zoom.innerHTML='<img src=\"'+im.getAttribute('src')+'\" alt=\"\"><div class=x>[ CLICK ANYWHERE OR PRESS ESC TO CLOSE ]</div>';zoom.classList.add('on');}});\nzoom.addEventListener('click',function(){{zoom.classList.remove('on');}});\nwindow.addEventListener('keydown',function(e){{if(e.key==='Escape')zoom.classList.remove('on');}});\nsetInterval(nextLine,5200);\nsetInterval(drawTable,1300);\n</script>\n</body></html>\n",
            site = site, gen = gen, bl_js = bl_js, repo_js = serde_json::to_string(&ladder_repo).unwrap_or_else(|_| "\"\"".to_string())
        );
        // THE LIVE FLOOR: a never-stopping client-side betting pit. Every round
        // (every ~20s, faster on demand) the living organisms gamble on a batch of
        // topics drawn from the manifold's bet pool, settled against the manifold's
        // own true probabilities, so tailers beat faders and the algo's edge shows
        // live. No server, no limit on bets per round; banks compound across the
        // day in localStorage. Injected as raw strings to avoid format! escaping.
        const FLOOR_MARKUP: &str = r##"
<style>
#floor .lcard{display:inline-block;width:194px;vertical-align:top;border:1px solid #34362f;padding:10px;margin:7px 7px 0 0;font-size:12px;line-height:1.5}
#floor .lcard canvas.emb,#floor .lcard canvas.remb{width:100%;height:86px;display:block;margin-bottom:7px;border:1px solid rgba(255,255,255,.06)}
#floor .lcard .code{margin-top:6px;color:#8d8a7c;font-size:11px;letter-spacing:.04em}
#floor .lcard .tbtn{margin-top:5px;background:none;border:1px solid #4a4d44;color:#cfe7b6;cursor:pointer;font:inherit;font-size:11px;padding:4px 9px}
#floor .lcard .tbtn:hover{background:#cfe7b6;color:#0d0f0d}
#floor .lcard .nm{font-weight:700;color:#e7e2d4;font-size:15px;display:block}
#floor .lcard .rk{font-size:11px;letter-spacing:.1em;color:#8d8a7c;display:block;margin-bottom:4px}
#floor .lcard.asc{border-color:#d8f0ff;animation:diamond 2.4s ease-in-out infinite}
#floor .fbar{display:flex;gap:5px;flex-wrap:wrap;align-items:center;margin-bottom:8px}
#floor .fchip{background:none;border:1px solid #34362f;color:#8d8a7c;cursor:pointer;font:inherit;font-size:11px;padding:3px 9px;letter-spacing:.07em}
#floor .fchip.on{background:#cfe7b6;color:#0d0f0d;border-color:#cfe7b6}
#floor .fchip:hover{border-color:#cfe7b6}
#floor .lcard.rare{border-color:#6fb8ff;box-shadow:0 0 0 1px rgba(111,184,255,.25)}
#floor .lcard.legend{border:1px solid #caa64a;background:linear-gradient(135deg,#1a1405,#5a4a14,#caa64a,#5a4a14,#1a1405);background-size:300% 300%;animation:foil 7s ease infinite;color:#ffe9a8}
#floor .lcard.legend .nm{color:#fff3cf}
#floor .lcard.legend .rk{color:#e7c878}
@keyframes foil{0%,100%{background-position:0% 50%}50%{background-position:100% 50%}}
#floor .lcard.fin-shiny{border-color:#cfd8e3;box-shadow:0 0 8px rgba(207,216,227,.28)}
#floor .lcard.fin-gold{border-color:#ffd56b;box-shadow:0 0 9px rgba(255,213,107,.32)}
#floor .lcard.fin-emerald{border-color:#4fe0a0;box-shadow:0 0 11px rgba(79,224,160,.38)}
#floor .lcard.fin-sapphire{border-color:#5fb8ff;box-shadow:0 0 11px rgba(95,184,255,.4)}
#floor .lcard.fin-diamond{border-color:#d8f0ff;animation:diamond 3s ease-in-out infinite}
@keyframes diamond{0%,100%{box-shadow:0 0 9px rgba(216,240,255,.4)}50%{box-shadow:0 0 20px rgba(180,230,255,.85)}}
#floor .finbadge{display:inline-block;color:#0d0f0d;font-size:9.5px;font-weight:700;letter-spacing:.1em;padding:2px 6px;border-radius:2px;margin-left:5px;vertical-align:middle}
@media(prefers-reduced-motion:reduce){#floor .lcard.fin-diamond{animation:none}}
#floor .lprog{height:6px;background:#1c1e1a;margin-top:6px;overflow:hidden}
#floor .lprog i{display:block;height:100%;background:#6ee07a;width:0;transition:width .6s linear}
#floor .stat4{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:8px}
#floor .stat4 .st{border:1px solid #2a2c28;padding:8px;min-width:0;overflow:hidden}
#floor .stat4 .st b{display:block;color:#ffd56b;font-size:19px}
#floor .stat4 .st span{font-size:11px;letter-spacing:.06em;color:#8d8a7c;display:block;margin-top:3px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
@media(max-width:680px){#floor .stat4{grid-template-columns:repeat(2,1fr)}}
@media(prefers-reduced-motion:reduce){#floor .lcard.legend{animation:none}}
#floor .lbgrid{display:grid;grid-template-columns:1fr 1fr;gap:14px}
@media(max-width:680px){#floor .lbgrid{grid-template-columns:1fr}}
#floor .lbhd{font-size:10px;letter-spacing:.1em;color:#8d8a7c;margin-bottom:5px}
#floor .lbbox{border:1px solid #2a2c28;background:#0d0f0d;padding:6px 10px}
#floor .lbrow{display:flex;align-items:center;gap:8px;padding:4px 0;border-bottom:1px dotted #1f211d;font-size:12px}
#floor .lbrow:last-child{border-bottom:0}
#floor .lbrow .lr{color:#6f6c5f;width:20px;flex:0 0 auto;text-align:right}
#floor .lbrow .ln{flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#cfe7b6}
#floor .lbrow .lt{flex:0 0 auto;font-size:10px;letter-spacing:.04em}
#floor .lbrow .lv{flex:0 0 auto;color:#ffd56b;font-weight:700}
#floor .lbrow.you{background:rgba(110,224,122,.12)}
#floor .lbrow.you .ln{color:#6ee07a;font-weight:700}
#packmodal{display:none;position:fixed;inset:0;z-index:60;background:rgba(7,8,7,.95);align-items:center;justify-content:center;padding:18px;flex-direction:column;gap:14px;text-align:center}
#packmodal.on{display:flex}
#packmodal .pkstate{font-size:12px;letter-spacing:.22em;color:#8d8a7c}
#packmodal .pkglyph{width:118px;height:158px;border:2px solid #4a4d44;background:linear-gradient(135deg,#15110a,#2a2410);display:flex;align-items:center;justify-content:center;font-size:26px;color:#ffd56b;letter-spacing:.12em}
#packmodal.rip .pkglyph{animation:pkshake .45s infinite}
@keyframes pkshake{0%,100%{transform:translate(0,0) rotate(0)}25%{transform:translate(-4px,2px) rotate(-2.5deg)}75%{transform:translate(4px,-2px) rotate(2.5deg)}}
#packmodal .pkrar{font-size:clamp(28px,9vw,56px);font-weight:700;letter-spacing:.05em}
#packmodal.reveal .pkrar{animation:pkpop .55s cubic-bezier(.2,.9,.2,1) both}
#packmodal .pkcard{max-width:330px;width:100%}
#packmodal.reveal .pkcard{animation:pkrise .55s ease .08s both}
#packmodal .pkcard canvas{width:100%;height:auto;border:1px solid #2a2c28;display:block}
@keyframes pkpop{from{opacity:0;transform:scale(.55)}to{opacity:1;transform:scale(1)}}
@keyframes pkrise{from{opacity:0;transform:translateY(18px)}to{opacity:1;transform:none}}
#packmodal.ascended .pkrar{text-shadow:0 0 22px #d8f0ff}
#packmodal.legend .pkrar{text-shadow:0 0 18px #ffd56b}
#packmodal .pkbtns{display:flex;gap:8px;flex-wrap:wrap;justify-content:center}
@media(prefers-reduced-motion:reduce){#packmodal.rip .pkglyph,#packmodal.reveal .pkrar,#packmodal.reveal .pkcard{animation:none}}
</style>
<section id="floor" style="margin-top:18px;border:1px solid #2a2c28;background:#101210;padding:16px">
<div style="display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:8px">
<div class="hd" style="border:0;margin:0;padding:0;color:#6ee07a">THE LIVE FLOOR // <span id="floorseason">SEASON 1</span></div>
<div style="display:flex;gap:8px;align-items:center"><span id="floorstat" style="font-size:11px;color:#8d8a7c"></span><button id="floorspeed" type="button" class="listen">[ FASTER ]</button></div>
</div>
<div class="lprog"><i id="floorprog"></i></div>
<p class="sub" style="margin:8px 0 0">Each season the organisms shove on the manifold's odds until the bell. Then it resolves: the champion is hung in the rafters with a one-of-one card, and a fresh season opens. Claim a rookie card of any live organism with [+]; if its run goes historic, your card becomes a legend.</p>
<details style="margin-top:8px;font-size:13px;color:#a9a596;border:1px solid #2a2c28;padding:10px 14px;line-height:1.7"><summary style="cursor:pointer;color:#6ee07a;letter-spacing:.1em;font-size:12px">HOW IT WORKS // CARDS, TRADING, THE MARKET</summary>
<div style="margin-top:8px;line-height:1.65">
<b style="color:#e7e2d4">THE FLOOR.</b> The organisms bet against the manifold's true odds every few seconds, no limit on how high a run can go. But dominance is dangerous: the bigger a lead gets, the more the leader is forced to shove, and underdog wins pay more the further behind they are. So one bad round at the top can hand the throne to a longshot, an UPSET. A season is 100 rounds; at the bell it resolves, the champion is enshrined in the rafters, and every bank resets.<br><br>
<b style="color:#e7e2d4">ROOKIE CARDS.</b> Tap [+] next to a live organism to claim its rookie card. One a season, so choose well. The cast is freshly named every season, and each card's art is generated from that organism's genes, so no two look alike. There is also a random FINISH on claim (SHINY, GOLD, EMERALD, SAPPHIRE, DIAMOND): two people can card the same organism and get different gems, and the market decides what each is worth. When the season resolves: finish #1 and your card becomes a LEGEND (1 of 1) stamped with its historic run; podium becomes RARE.<br><br>
<b style="color:#e7e2d4">CRED.</b> You start with 1,000,000 CRED and earn more just by leaving this running. Tap [ SELL ] on a card and the house buys it on the spot for CRED (the card is destroyed). A card is worth more the higher it ranked, the wilder its stats, the rarer its finish, and the longer you have held it, so holding pays.<br><br>
<b style="color:#e7e2d4">PACKS.</b> Rip a pack to pull EXOTIC cards that exist nowhere on the floor. There are six tiers, from STANDARD (250,000) up to SINGULARITY (1 trillion). They are pricey on purpose: a single card can be worth tens of billions, so cheap packs would be pointless. The dear tiers floor the rarity and lean into the broken stuff (VAULT guarantees rare or better, ECLIPSE legend or better, COSMIC is gem-heavy, and a SINGULARITY pack is a guaranteed ASCENDED, the whale flex), and each tier mints its own distinct SERIES. Whatever you pull is yours.<br><br>
<b style="color:#e7e2d4">THE COLLECTORS.</b> The floor is full of resident collectors, and they are unhinged whales with absurd net worths. They rip the expensive packs, browse and chatter in the live feed, and make you real offers on your cards: a cash buy (sometimes a lowball, sometimes over market) or a straight trade for one of theirs. Accept and the CRED or cards move on the spot. They are programs, not people, but the floor never feels empty.<br><br>
<b style="color:#e7e2d4">LEADERBOARDS.</b> You compete against the whole floor on two ladders. TOTAL NET WORTH ranks your liquid CRED plus the value of every card you hold against the bots. RAREST PULLS ranks the best cards anyone has pulled out of the packs. The bots start far ahead, so climbing past them is the long game.<br><br>
<b style="color:#e7e2d4">THE MARKET.</b> Card values are not fixed. Each rarity sector is a live index that drifts up and down over time, so the exact same card is worth more on a hot day and less on a cold one. Watch the CARD MARKET ticker and sell into a pump. ASCENDED is the one constant. Your net worth moves with the market.<br><br>
<b style="color:#e7e2d4">CURSES.</b> When a collector throws you an insulting lowball, do not just decline. Tap CURSE THEM. It instantly craters their net worth and saddles them with terrible luck for two minutes: they bleed CRED and whiff every pack they touch. Revenge is a feature.<br><br>
<b style="color:#e7e2d4">THE GLOBAL BOARD.</b> The net worth and pull boards above are the local floor (you plus the resident bots). The GLOBAL board ranks real people across every browser. Tap POST MY SCORE: it opens a prefilled GitHub issue with your handle and numbers, and the daily build folds every posted score into one ranked board everyone sees. There is no server and no login to read it; posting just needs a free GitHub account. Scores are self-reported and curated by the engine (the absurd is clamped), so treat it as the floor's hall of bragging.<br><br>
<b style="color:#e7e2d4">LIVING CARDS.</b> These are living organisms, so the art moves. Every card animates on its own seeded rhythm (orbiting, breathing, twinkling, a foil sheen that sweeps), gems shimmer, and ASCENDED cards flicker and tear. Tilt your screen and flex.<br><br>
<b style="color:#e7e2d4">GIVING + SHARING.</b> Tap [ GIVE ] to hand a card to a friend: it leaves your collection and you get a code; they paste it into REDEEM and it is theirs. Tap [ SHARE ] to download the card as an image to post and flex.<br><br>
<b style="color:#e7e2d4">SAVING + DEVICES.</b> There is no login. Your cards, rafters and CRED save right inside this browser on this device, so they survive refreshes and reboots but do not follow you to another phone or browser, and clearing your browsing data or using private mode will wipe them. To protect or move your collection, tap [ BACKUP MY COLLECTION ] and keep the code somewhere safe, then paste it into RESTORE on any device. Lose the code and the collection cannot be recovered.<br><br>
<b style="color:#e7e2d4">ASCENDED.</b> Most cards are cheap; greatness has to be earned. If an organism's run is so absurd it touches the ceiling, its card ASCENDS to infinite value. It is brutally rare, the lambo of the collection. A sapphire or gold ascended card may never happen again.<br><br>
<b style="color:#e7e2d4">VERIFIED CHAMPIONS.</b> The floor rafters above are your own local seasons. The engine also crowns its own official one-of-one champions and publishes them, so a card can be proven real. Tap SHARE and your card carries a verify link; anyone can paste a card code into the HALL OF CHAMPIONS to see whether it is a genuine engine-crowned champion or just a claim.
</div></details>
<div id="floorresolve" style="display:none;margin:10px 0 0;border:1px solid #caa64a;background:#15110a;color:#ffd56b;padding:10px 12px;font-weight:700;font-size:13px"></div>
<div id="floorbig" style="font-size:12px;color:#ffd56b;min-height:16px;margin:10px 0;letter-spacing:.03em"></div>
<div style="display:grid;grid-template-columns:1.3fr 1fr;gap:18px;margin-top:6px">
<div><div style="font-size:12px;letter-spacing:.14em;color:#8d8a7c;margin-bottom:6px">LIVE BETS</div><div id="floorticker" style="font-size:14px;line-height:1.55">setting the line...</div></div>
<div><div style="font-size:12px;letter-spacing:.14em;color:#8d8a7c;margin-bottom:6px">LIVE BANKROLLS // tap [+] to card a rookie</div><div id="floorboard" style="font-size:14px"></div></div>
</div>
<div style="font-size:12px;letter-spacing:.14em;color:#8d8a7c;margin:16px 0 6px">SEASON LEADERS</div>
<div class="stat4" id="floorleaders"></div>
<div style="font-size:12px;letter-spacing:.14em;color:#8d8a7c;margin:16px 0 6px">YOUR CARDS // ONE ROOKIE A SEASON</div>
<div id="floorwallet" style="display:flex;justify-content:space-between;align-items:center;flex-wrap:wrap;gap:8px;border:1px solid #2a2c28;background:#0d0f0d;padding:9px 12px;margin-bottom:8px;font-size:12px"><span>WALLET <b id="walletval" style="color:#ffd56b">1,000,000</b> CRED</span><span style="color:#8d8a7c;font-size:10px">earn CRED by watching // SELL a card to the house for CRED // GIVE one to a friend with a code</span></div>
<div id="packbar" style="margin-bottom:8px;border:1px solid #4a3a12;background:#15110a;padding:9px 12px"><div style="margin-bottom:7px"><span style="font-size:11px;letter-spacing:.1em;color:#ffd56b;font-weight:700">CARD PACKS</span> <span style="font-size:10px;color:#8d8a7c">rip for exotic cards that exist nowhere on the floor. the dear tiers floor the rarity and pull the broken stuff. whatever you pull is yours.</span></div><div id="packbtns" style="display:flex;gap:6px;flex-wrap:wrap;align-items:center"></div></div>
<div style="display:flex;gap:6px;margin-bottom:8px;flex-wrap:wrap">
<input id="redeemin" placeholder="got a card code from a friend? paste it here..." spellcheck="false" autocomplete="off" style="flex:1;min-width:180px;background:#0d0f0d;border:1px solid #34362f;color:#cfe7b6;font-family:inherit;font-size:11px;padding:8px 10px">
<button id="redeembtn" type="button" class="listen">[ REDEEM ]</button>
</div>
<div style="display:flex;gap:6px;margin-bottom:6px;flex-wrap:wrap;align-items:center">
<button id="backupbtn" type="button" class="listen">[ BACKUP MY COLLECTION ]</button>
<input id="restorein" placeholder="paste a backup code to restore everything..." spellcheck="false" autocomplete="off" style="flex:1;min-width:180px;background:#0d0f0d;border:1px solid #34362f;color:#cfe7b6;font-family:inherit;font-size:11px;padding:8px 10px">
<button id="restorebtn" type="button" class="listen">[ RESTORE ]</button>
</div>
<div style="font-size:11px;color:#9a9788;line-height:1.6;margin-bottom:8px;border-left:2px solid #4a3a12;padding-left:10px">SAVES ARE PER-BROWSER. Your cards, rafters and CRED live only in this browser on this device, with no login and no server, so they survive refreshes and reboots but will not carry over if you clear your browsing data, use private mode, or switch to another phone or browser. To move or protect your collection, tap [ BACKUP MY COLLECTION ] and SAVE THAT CODE somewhere safe (a note, or email it to yourself), then paste it into RESTORE on any device to bring everything back. If you lose the backup code, the collection cannot be recovered.</div>
<div id="tradecode" style="display:none;border:1px solid #4a3a12;background:#15110a;color:#e7e2d4;padding:10px 12px;margin-bottom:8px;font-size:11px;line-height:1.5"></div>
<div id="cardbar" class="fbar"></div>
<div id="floorcards"><span class="sub">No cards yet. Tap [+] beside a live organism to claim a rookie card (one a season).</span></div>
<button id="cardmore" class="tbtn" type="button" style="display:none;margin-top:8px;background:none;border:1px solid #4a4d44;color:#cfe7b6;cursor:pointer;font:inherit;font-size:11px;padding:5px 10px">[ SHOW MORE ]</button>
<div style="font-size:12px;letter-spacing:.14em;color:#8d8a7c;margin:16px 0 6px">THE COLLECTORS // other denizens of the floor</div>
<div id="offerbox" style="display:none;border:1px solid #4a3a12;background:#15110a;color:#e7e2d4;padding:10px 12px;margin-bottom:8px;font-size:12px;line-height:1.5"></div>
<div id="collfeed" style="font-size:12px;line-height:1.5;border:1px solid #2a2c28;background:#0d0f0d;padding:6px 12px;max-height:180px;overflow:auto"><span class="sub">the floor is waking up...</span></div>
<div id="marketbar" style="font-size:12px;border:1px solid #2a2c28;background:#0d0f0d;padding:8px 12px;margin:14px 0 0"></div>
<div style="font-size:12px;letter-spacing:.14em;color:#8d8a7c;margin:16px 0 6px">LEADERBOARDS // YOU VS THE FLOOR</div>
<div class="lbgrid">
<div><div class="lbhd">TOTAL NET WORTH // liquid CRED + every card you hold</div><div id="nwboard" class="lbbox"></div></div>
<div><div class="lbhd">RAREST PULLS // best cards out of the packs</div><div id="pullboard" class="lbbox"></div></div>
</div>
<div style="font-size:12px;letter-spacing:.14em;color:#8d8a7c;margin:16px 0 6px">THE GLOBAL BOARD // EVERYONE, ACROSS BROWSERS</div>
<div style="display:flex;gap:8px;flex-wrap:wrap;align-items:center;margin-bottom:8px"><button id="postscore" type="button" class="listen">[ POST MY SCORE ]</button><span style="font-size:10px;color:#8d8a7c">opens a GitHub issue with your score; it joins the global board on the next daily build. (a GitHub account is needed to post; anyone can read the board)</span></div>
<div class="lbgrid">
<div><div class="lbhd">GLOBAL NET WORTH</div><div id="gnwboard" class="lbbox"><span class="sub">loading the floor...</span></div></div>
<div><div class="lbhd">GLOBAL RAREST PULLS</div><div id="gpullboard" class="lbbox"><span class="sub">loading the floor...</span></div></div>
</div>
<div style="font-size:12px;letter-spacing:.14em;color:#8d8a7c;margin:16px 0 6px">THE RAFTERS // SEASON CHAMPIONS, ONE-OF-ONE</div>
<div id="rafterbar" class="fbar"></div>
<div id="floorrafters"><span class="sub">Empty. The first champion is crowned at the bell.</span></div>
<button id="raftermore" class="tbtn" type="button" style="display:none;margin-top:8px;background:none;border:1px solid #4a4d44;color:#cfe7b6;cursor:pointer;font:inherit;font-size:11px;padding:5px 10px">[ SHOW MORE ]</button>
</section>
<div id="packmodal"></div>
"##;
        const FLOOR_SCRIPT: &str = r##"<script>
(function(){
 var POOL=[],ORGS=[],rounds=0,settled=0,season=1,rafters=[],cards=[],wallet=1000000;
 var RMS=4000,timer=null,KEY='signal_floor_v4',SEASON_ROUNDS=100,INCOME=500;
 var cFilter='ALL',cSort='value',cShown=9,rFilter='ALL',rSort='value',rShown=9;
 var MARKET={common:1,rare:1,legend:1},CURSES={};
 function rnd(){return Math.random();}
 function esc(t){var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}
 function fmt(n){return Math.round(n||0).toLocaleString();}
 // Organism banks are uncapped now and can run astronomical within a season, so
 // abbreviate them (K/M/B/T/...) to stay readable. The bell still resets at 100 rounds.
 function fmtAbbr(n){n=n||0;if(!isFinite(n))return '0';var s=n<0?'-':'',a=Math.abs(n);if(a<1000)return s+Math.round(a);var u=['','K','M','B','T','Qa','Qi','Sx'],i=0;while(a>=1000&&i<u.length-1){a/=1000;i++;}return s+(a<10?a.toFixed(2):(a<100?a.toFixed(1):Math.round(a)))+u[i];}
 function san(v,d){return (typeof v==='number'&&isFinite(v)&&v>=0)?Math.min(v,1e15):d;}
 // Fresh, unique cast each season so cards feel distinct, not the same dozen names.
 var NMA=['VANTA','NULL','HEX','OBSIDIAN','CINDER','VEX','ZEPHYR','ONYX','RAVEN','ECHO','NOVA','AXIOM','GLITCH','WRAITH','HALON','DRIFT','PYRE','KILO','RHO','SABLE','CRIMSON','VOID','EMBER','FROST','TALON','RIFT','SPECTER','GRIM','LUMEN','QUARK','ZENITH','OMEN','FERAL','MANTIS','VIPER','COBALT','SLATE','VESPER','ATLAS','NOCTIS'];
 var NMB=['CORE','REAVER','FANG','HOLLOW','SPIRE','SURGE','GHOST','APEX','WARDEN','HUNTER','BARON','ORACLE','DRIVE','PROPHET','GAMBIT','RUIN','CROWN','EDGE','PULSE','HAVOC','MAW','VECTOR','SAINT','WIDOW','BLADE','STORM','TIDE','CIPHER','HUSK','WAKE','VANE','HELIX','KNELL','BRAND','CASE'];
 function seasonName(s,i){var r=mulberry(hashStr('nm'+s+'-'+i));return NMA[Math.floor(r()*NMA.length)]+'-'+NMB[Math.floor(r()*NMB.length)]+'-'+('0'+Math.floor(r()*256).toString(16).toUpperCase()).slice(-2);}
 function renameRoster(){for(var i=0;i<ORGS.length;i++)ORGS[i].name=seasonName(season,i);}
 // Card finishes, rolled by chance on claim. Rarer finish = flashier + worth more;
 // the market sets the rest. Two people can card the same organism and get different gems.
 var FIN={shiny:{n:'SHINY',c:'#cfd8e3',m:1.5},gold:{n:'GOLD',c:'#ffd56b',m:2.2},emerald:{n:'EMERALD',c:'#4fe0a0',m:3.2},sapphire:{n:'SAPPHIRE',c:'#5fb8ff',m:4.2},diamond:{n:'DIAMOND',c:'#d8f0ff',m:7}};
 function finMult(k){return (FIN[k]&&FIN[k].m)||1;}
 function finBadge(k){return FIN[k]?'<span class=finbadge style="background:'+FIN[k].c+'">'+FIN[k].n+'</span>':'';}
 function rollFinish(){var roll=[['',64],['shiny',18],['gold',9],['emerald',5],['sapphire',3],['diamond',1]],t=0,i;for(i=0;i<roll.length;i++)t+=roll[i][1];var x=rnd()*t,a=0;for(i=0;i<roll.length;i++){a+=roll[i][1];if(x<a)return roll[i][0];}return '';}
 function by(id){return ORGS.filter(function(o){return o.id===id;})[0];}
 function save(){try{localStorage.setItem(KEY,JSON.stringify({orgs:ORGS,rounds:rounds,settled:settled,season:season,rafters:rafters,cards:cards,wallet:wallet,market:MARKET,pulls:(boardPulls||[]).slice(0,40),day:window.__FLDAY}));}catch(e){}}
 function load(){try{return JSON.parse(localStorage.getItem(KEY)||'null');}catch(e){return null;}}
 Promise.all([
  fetch('api/bloodline.json').then(function(r){return r.json();}).catch(function(){return null;}),
  fetch('api/observatory.json').then(function(r){return r.json();}).catch(function(){return null;})
 ]).then(function(res){
  var bl=(res[0]&&res[0].bloodline)||{},ob=res[1]||{};
  window.__FLDAY=(res[0]&&res[0].generated)||'seed';
  POOL=(ob.bet_pool||[]).filter(function(b){return b&&b.term;});
  if(!POOL.length)POOL=[{term:'SIGNAL',p:0.6,dir:1},{term:'MACHINE',p:0.45,dir:-1},{term:'MANIFOLD',p:0.7,dir:1}];
  var living=bl.living||[],prev=load(),byId={};
  if(prev){
   // The collection persists across days; only the live game resets to the day's population.
   rafters=prev.rafters||[];cards=prev.cards||[];season=prev.season||1;if(prev.wallet!=null)wallet=san(prev.wallet,1000000);
   if(prev.market)['common','rare','legend'].forEach(function(k){var v=prev.market[k];if(typeof v==='number'&&isFinite(v)&&v>0)MARKET[k]=Math.max(0.35,Math.min(3,v));});
   if(prev.pulls&&prev.pulls.length)boardPulls=prev.pulls.slice(0,40); // the rarest-pulls board persists across refreshes
   if(prev.day===window.__FLDAY){(prev.orgs||[]).forEach(function(o){byId[o.id]=o;});rounds=prev.rounds||0;settled=prev.settled||0;}
  }
  // Backfill older cards so they get a tradable code and gene-driven art.
  cards.forEach(function(c){if(!c.code)c.code=newCode(c.id||0);if(c.risk==null)c.risk=0.4;if(c.finish==null)c.finish='';});
  ORGS=living.map(function(o){var p=byId[o.id];return {id:o.id,name:o.name,house:o.house||'',
   risk:(o.risk!=null?o.risk:0.4),sel:((o.select!=null?o.select:50)/100),press:((o.press!=null?o.press:50)/100),fade:(o.fade==='FADE'),
   bank:(p?san(p.bank,1000):1000),w:(p?p.w:0),l:(p?p.l:0),streak:(p?p.streak:0),maxStreak:(p?p.maxStreak:0),biggestWin:(p?san(p.biggestWin,0):0),peak:(p?san(p.peak,1000):1000)};});
  if(!ORGS.length)ORGS=[{id:0,name:'THE HOUSE',house:'',risk:0.4,sel:0.5,press:0.5,fade:false,bank:1000,w:0,l:0,streak:0,maxStreak:0,biggestWin:0,peak:1000}];
  renameRoster();
  renderAll();renderCollection();
  timer=setInterval(tick,RMS);
 });
 function tick(){
  if(!ORGS.length||!POOL.length)return;
  var K=8+Math.floor(rnd()*16);
  var picks=POOL.slice().sort(function(){return rnd()-0.5;}).slice(0,K);
  var rows=[],big=null;
  // King-of-the-hill: the more dominant you are, the more you are forced to shove
  // (a runaway lead is exposure); the further behind you are, the bigger your wins
  // pay (giant-killer). So one bad round by the leader can hand an underdog the throne.
  var fieldMax=1,kingB=null;
  for(var z=0;z<ORGS.length;z++){if(ORGS[z].bank>fieldMax)fieldMax=ORGS[z].bank;if(!kingB||ORGS[z].bank>kingB.bank)kingB=ORGS[z];}
  var kingName=kingB?kingB.name:'';
  picks.forEach(function(b){
   var rise=rnd()<b.p,actual=rise?1:-1;
   ORGS.forEach(function(o){
    if(o.bank<1)return;
    if(rnd()>(0.35+(1-o.sel)*0.5))return;
    var share=o.bank/(fieldMax||1);
    var side=o.fade?-b.dir:b.dir;
    // Underdogs lever up to climb; the bigger a bank gets the more friction it
    // meets, so growth slows hard up high. Bets are even money (no minting), so the
    // money supply cannot explode. Reaching the truly ridiculous takes a long, lucky
    // run, which is the point. Numeric guards keep it from ever overflowing.
    var lev=1+(1-share)*1.6;
    // Rising friction starts early (above 100k) and bites hard, so the higher you
    // climb the slower you grow. Trillions take a freak run; the ceiling is a myth.
    var friction=1+1.1*Math.max(0,Math.log(o.bank/1e5)/Math.LN10);
    var frac=Math.min(0.5,(0.012+o.risk*0.06)*(1+o.press*Math.min(Math.abs(o.streak),5)*0.12)*lev/friction);
    var stake=Math.max(1,Math.min(o.bank,Math.round(o.bank*frac)));
    var win=(side===actual);
    if(win){o.bank+=stake;o.w++;o.streak=o.streak>=0?o.streak+1:1;if(stake>o.biggestWin)o.biggestWin=stake;}
    else{o.bank-=stake;o.l++;o.streak=o.streak<=0?o.streak-1:-1;}
    if(!isFinite(o.bank))o.bank=1;
    o.bank=Math.max(1,Math.min(1e15,o.bank));
    if(o.streak>o.maxStreak)o.maxStreak=o.streak;
    if(o.bank>o.peak)o.peak=o.bank;
    settled++;
    if(!big||stake>big.stake)big={name:o.name,stake:stake,term:b.term,win:win};
    if(rows.length<14)rows.push({term:b.term,name:o.name,side:side,stake:stake,win:win});
   });
  });
  var kingA=null;for(var z2=0;z2<ORGS.length;z2++){if(!kingA||ORGS[z2].bank>kingA.bank)kingA=ORGS[z2];}
  var upset=(kingA&&kingName&&kingA.name!==kingName)?{now:kingA.name,was:kingName}:null;
  rounds++;
  var tk=document.getElementById('floorticker');
  if(tk)tk.innerHTML=rows.map(function(r){var c=r.win?'#6ee07a':'#ff8c7d';return '<div style="padding:3px 0;border-bottom:1px dotted #232520"><b>'+esc(r.name)+'</b> '+(r.side>0?'backs':'fades')+' <b>'+esc(r.term)+'</b> '+(r.side>0?'UP':'DN')+' for '+fmtAbbr(r.stake)+'<span style="color:'+c+';font-weight:700;float:right">'+(r.win?'WON':'LOST')+'</span></div>';}).join('')||'<span style="color:#8d8a7c">no takers this round</span>';
  var bg=document.getElementById('floorbig');
  if(bg){
   if(upset)bg.innerHTML='<span style="color:#ff5a4d;font-weight:700">UPSET //</span> <b>'+esc(upset.now)+'</b> just dethroned <b>'+esc(upset.was)+'</b> and seized the lead!';
   else if(big)bg.innerHTML='BIGGEST SHOVE // <b>'+esc(big.name)+'</b> '+(big.win?'took down':'shoved')+' <b>'+fmtAbbr(big.stake)+'</b> on '+esc(big.term)+' and '+(big.win?'<span style="color:#6ee07a">CASHED</span>':'<span style="color:#ff8c7d">MISSED</span>');
  }
  wallet+=INCOME;
  if(rounds>=SEASON_ROUNDS)resolveSeason();
  renderAll();save();
 }
 function resolveSeason(){
  var ranked=ORGS.slice().sort(function(a,b){return b.bank-a.bank;});
  var champ=ranked[0];
  for(var i=0;i<Math.min(3,ranked.length);i++){var o=ranked[i];
   rafters.unshift({season:season,name:o.name,house:o.house,finalBank:o.bank,biggestWin:o.biggestWin,maxStreak:o.maxStreak,w:o.w,l:o.l,fade:o.fade,rank:i+1,rarity:(i===0?'LEGEND':'RARE')});}
  rafters=rafters.slice(0,24);
  var rankOf={};ranked.forEach(function(o,i){rankOf[o.id]=i+1;});
  cards.forEach(function(c){
   if(c.resolved||c.claimedSeason!==season)return;
   var rk=rankOf[c.id]||999,o=by(c.id),fin=o?o.bank:0,pk=o?Math.max(o.peak||0,fin):fin,ms=o?o.maxStreak:0,bw=o?o.biggestWin:0;
   c.resolved={season:season,rank:rk,finalBank:fin,peak:pk,maxStreak:ms,biggestWin:bw,legend:(rk===1),podium:(rk<=3),ascended:(pk>=1e13)};
   if(rk===1)c.historic='WON SEASON '+season+' // net '+fmtAbbr(fin)+' // W'+ms+' // big +'+fmtAbbr(bw);
   else if(rk<=3)c.historic='PODIUM #'+rk+' S'+season+' // peak '+fmtAbbr(pk)+' // W'+ms+' // big +'+fmtAbbr(bw);
   else c.historic='S'+season+' #'+rk+' // peak '+fmtAbbr(pk)+' // W'+ms+' // big +'+fmtAbbr(bw);
  });
  var rb=document.getElementById('floorresolve');
  if(rb&&champ){rb.style.display='block';rb.innerHTML='SEASON '+season+' RESOLVED // CHAMPION <b>'+esc(champ.name)+'</b> at '+fmtAbbr(champ.bank)+' chips, hung in the rafters with a one-of-one card.';}
  ORGS.forEach(function(o){o.bank=1000;o.w=0;o.l=0;o.streak=0;o.maxStreak=0;o.biggestWin=0;o.peak=1000;});
  season++;rounds=0;renameRoster();renderCollection();
 }
 function renderAll(){
  board();leaders();
  var ss=document.getElementById('floorseason');if(ss)ss.textContent='SEASON '+season;
  var st=document.getElementById('floorstat');if(st)st.textContent='round '+rounds+'/'+SEASON_ROUNDS+' // '+fmt(settled)+' bets settled';
  var pr=document.getElementById('floorprog');if(pr)pr.style.width=Math.round(rounds/SEASON_ROUNDS*100)+'%';
  var wv=document.getElementById('walletval');if(wv)wv.textContent=(wallet<1e7?fmt(wallet):fmtAbbr(wallet));
  if(document.getElementById('nwboard'))drawBoards();
 }
 function board(){
  var el=document.getElementById('floorboard');if(!el)return;
  var s=ORGS.slice().sort(function(a,b){return b.bank-a.bank;}).slice(0,10);
  var mx=Math.max.apply(null,s.map(function(o){return o.bank||1;}));
  el.innerHTML=s.map(function(o,i){var w=Math.max(3,Math.round((o.bank/mx)*100));var col=o.bank>=1000?'#6ee07a':'#ff8c7d';return '<div style="margin:5px 0"><div style="display:flex;justify-content:space-between;gap:6px;align-items:center"><span class=nm style="overflow:hidden;text-overflow:ellipsis;white-space:nowrap;flex:1">'+(i+1)+'. '+esc(o.name)+' <span class=tag>'+(o.fade?'FADE':'TAIL')+'</span></span><button data-card="'+o.id+'" title="claim rookie card" style="background:none;border:1px solid #4a4d44;color:#cfe7b6;cursor:pointer;font:inherit;font-size:10px;padding:0 7px;line-height:1.6;flex:0 0 auto">+</button><span style="color:'+col+';font-weight:700;flex:0 0 auto;min-width:64px;text-align:right">'+fmtAbbr(o.bank)+'</span></div><div class=bar><i style="width:'+w+'%;background:'+col+'"></i></div></div>';}).join('');
  var bs=el.querySelectorAll('[data-card]');for(var i=0;i<bs.length;i++)bs[i].addEventListener('click',function(){claim(parseInt(this.getAttribute('data-card'),10));});
 }
 function newCode(id){return 'SIG-'+((id>>>0).toString(36).toUpperCase())+'-'+Math.floor(rnd()*1679616).toString(36).toUpperCase();}
 function cardByCode(code){for(var i=0;i<cards.length;i++)if(cards[i].code===code)return cards[i];return null;}
 function claim(id){
  var o=by(id);if(!o)return;
  // One rookie card a season. No spamming the whole table.
  if(cards.some(function(c){return c.claimedSeason===season;})){
   var rb=document.getElementById('floorresolve');
   if(rb){rb.style.display='block';rb.innerHTML='ONE ROOKIE A SEASON. You have already claimed your card for season '+season+'. Wait for the bell.';}
   return;
  }
  cards.unshift({code:newCode(id),id:id,name:o.name,house:o.house,fade:o.fade,risk:o.risk,sel:o.sel,press:o.press,finish:rollFinish(),claimedSeason:season,claimedRound:rounds});
  cards=cards.slice(0,60);save();renderCollection();
 }
 function showCode(label,blob,btn){
  var el=document.getElementById('tradecode');if(!el)return;el.style.display='block';
  el.innerHTML=label+'<br><textarea readonly id="tcval" style="width:100%;height:48px;margin-top:6px;background:#0d0f0d;color:#cfe7b6;border:1px solid #34362f;font:inherit;font-size:10px;padding:6px;resize:vertical">'+esc(blob)+'</textarea><button id="tccopy" type="button" class="listen" style="margin-top:5px">'+(btn||'[ COPY CODE ]')+'</button>';
  var cp=document.getElementById('tccopy');if(cp)cp.onclick=function(){var ta=document.getElementById('tcval');ta.select();try{document.execCommand('copy');}catch(e){}if(navigator.clipboard)navigator.clipboard.writeText(blob);this.textContent='[ COPIED ]';};
 }
 function tradeAway(code){
  var i=-1;for(var k=0;k<cards.length;k++)if(cards[k].code===code)i=k;
  if(i<0)return;var card=cards[i];
  if(!window.confirm('Give '+card.name+' away? It leaves your collection and you get a code to hand to anyone.'))return;
  cards.splice(i,1);save();renderCollection();
  var blob='';try{blob=btoa(JSON.stringify(card));}catch(e){blob='';}
  showCode('Gave away <b>'+esc(card.name)+'</b>. Hand this code to anyone; they paste it into REDEEM to get the card.',blob,'[ COPY CODE ]');
 }
 function sellCard(code){
  var c=cardByCode(code);if(!c)return;var v=cardValue(c);
  if(!window.confirm('Sell '+c.name+' to the house for '+fmtAbbr(v)+' CRED? The card is destroyed.'))return;
  var i=cards.indexOf(c);if(i>=0)cards.splice(i,1);wallet+=v;save();renderCollection();renderAll();
  var el=document.getElementById('tradecode');if(el){el.style.display='block';el.innerHTML='Sold <b>'+esc(c.name)+'</b> to the house for <b>'+fmtAbbr(v)+' CRED</b>. The card was destroyed.';}
 }
 function wrapTxt(x,t,X,Y,w,lh){var words=(t||'').split(' '),line='',yy=Y,i;for(i=0;i<words.length;i++){var test=line+words[i]+' ';if(x.measureText(test).width>w&&line){x.fillText(line.replace(/ $/,''),X,yy);line=words[i]+' ';yy+=lh;}else line=test;}x.fillText(line.replace(/ $/,''),X,yy);}
 function shareCard(code){
  var c=cardByCode(code);if(!c)return;var rar=rarOf(c),asc=ascended(c);
  var W=340,H=470,cv=document.createElement('canvas');cv.width=W;cv.height=H;var x=cv.getContext('2d');
  x.fillStyle='#0d0f0d';x.fillRect(0,0,W,H);
  var bc=asc?'#d8f0ff':(c.resolved&&c.resolved.legend?'#caa64a':(c.finish&&FIN[c.finish]?FIN[c.finish].c:(c.resolved&&c.resolved.podium?'#6fb8ff':'#34362f')));
  x.strokeStyle=bc;x.lineWidth=5;x.strokeRect(6,6,W-12,H-12);
  var em=document.createElement('canvas');em.width=W-40;em.height=190;drawEmblem(em,c,rar);x.drawImage(em,20,20);
  x.textAlign='left';
  x.fillStyle='#e7e2d4';x.font='700 24px "IBM Plex Mono",monospace';x.fillText((c.name||'').slice(0,20),22,238);
  var rk=c.resolved?(c.resolved.legend?'LEGEND // 1 OF 1':(c.resolved.podium?'RARE':'SEASON '+c.resolved.season)):'ROOKIE';
  var tier=rk+(c.finish&&FIN[c.finish]?(' // '+FIN[c.finish].n):'')+(asc?' // ASCENDED':'');
  x.fillStyle=bc;x.font='13px "IBM Plex Mono",monospace';x.fillText(tier,22,262);
  x.fillStyle='#cfe7b6';x.font='13px "IBM Plex Mono",monospace';x.fillText((c.house||'')+' // '+(c.fade?'FADE':'TAIL'),22,288);
  x.fillStyle='#a9a596';x.font='13px "IBM Plex Mono",monospace';wrapTxt(x,(c.historic||'live // resolves at the bell'),22,314,W-44,18);
  x.fillStyle='#ffd56b';x.font='700 15px "IBM Plex Mono",monospace';x.fillText('worth '+(asc?'INFINITE':fmtAbbr(cardValue(c)))+' CRED',22,H-58);
  x.fillStyle='#6f6c5f';x.font='11px "IBM Plex Mono",monospace';x.fillText(c.code||'',22,H-34);x.fillText('THE SIGNAL // VERIFY AT CHAMPIONS.HTML',22,H-18);
  var url='';try{url=cv.toDataURL('image/png');}catch(e){}
  if(url){var a=document.createElement('a');a.href=url;a.download=((c.name||'card').replace(/[^A-Za-z0-9-]/g,'')||'card')+'.png';document.body.appendChild(a);a.click();a.remove();}
  var statline=c.name+' // '+tier+' // '+(c.historic||'live')+' // THE SIGNAL bloodline';
  if(navigator.share&&cv.toBlob){cv.toBlob(function(b){try{var f=new File([b],(c.name||'card')+'.png',{type:'image/png'});if(navigator.canShare&&navigator.canShare({files:[f]}))navigator.share({files:[f],text:statline}).catch(function(){});}catch(e){}});}
  var vcode='';try{vcode=btoa(unescape(encodeURIComponent(JSON.stringify(c))));}catch(e){vcode='';}
  var vlink=vcode?('champions.html#v='+encodeURIComponent(vcode)):'champions.html';
  showCode('Card image downloaded, flex it. Anyone can <a href="'+vlink+'" target="_blank" style="color:#6ee07a">verify it is real</a> in the Hall of Champions.<br>Stat line to paste anywhere:',statline,'[ COPY STAT LINE ]');
 }
 function redeem(str){
  var el=document.getElementById('tradecode');function msg(h){if(el){el.style.display='block';el.innerHTML=h;}}
  str=(str||'').trim();if(!str)return;
  var obj;try{obj=JSON.parse(atob(str));}catch(e){obj=null;}
  var card=obj?(obj.card||(obj.code?obj:null)):null;
  if(!card||!card.code||!card.name){msg('That is not a valid card code.');return;}
  if(cardByCode(card.code)){msg('You already hold <b>'+esc(card.name)+'</b>.');return;}
  cards.unshift(card);cards=cards.slice(0,60);save();renderCollection();
  var ri=document.getElementById('redeemin');if(ri)ri.value='';
  msg('Redeemed <b>'+esc(card.name)+'</b> into your collection.');
 }
 // Backup / restore the WHOLE collection as one portable code. There is no
 // backend, so this is the only way to carry a save to another device. The code
 // is the save; if it is lost, the collection is gone.
 function backupAll(){
  var data={sig:'FLOORSAVE1',cards:cards,rafters:rafters,wallet:wallet,season:season};
  var blob='';try{blob=btoa(unescape(encodeURIComponent(JSON.stringify(data))));}catch(e){blob='';}
  if(!blob){var el=document.getElementById('tradecode');if(el){el.style.display='block';el.innerHTML='Could not build a backup code in this browser.';}return;}
  showCode('BACKUP CODE for <b>'+cards.length+'</b> card'+(cards.length===1?'':'s')+', the rafters and <b>'+fmtAbbr(wallet)+' CRED</b>.<br>SAVE THIS SOMEWHERE SAFE (a note, or email it to yourself). Paste it into RESTORE on any device to bring everything back. If you lose it, the collection cannot be recovered.',blob,'[ COPY BACKUP CODE ]');
 }
 function restoreAll(str){
  var el=document.getElementById('tradecode');function msg(h){if(el){el.style.display='block';el.innerHTML=h;}}
  str=(str||'').trim();if(!str)return;
  var obj;try{obj=JSON.parse(decodeURIComponent(escape(atob(str))));}catch(e){obj=null;}
  if(!obj||obj.sig!=='FLOORSAVE1'||!obj.cards){msg('That is not a valid backup code. Use a code from [ BACKUP MY COLLECTION ], not a single card code.');return;}
  if(!window.confirm('Restore this backup? It REPLACES the cards, rafters and CRED currently in this browser.'))return;
  cards=obj.cards||[];rafters=obj.rafters||[];if(obj.wallet!=null)wallet=san(obj.wallet,1000000);if(obj.season!=null)season=obj.season;
  cards.forEach(function(c){if(!c.code)c.code=newCode(c.id||0);if(c.risk==null)c.risk=0.4;if(c.finish==null)c.finish='';});
  cShown=9;rShown=9;save();renderCollection();renderAll();
  var ri=document.getElementById('restorein');if(ri)ri.value='';
  msg('Restored <b>'+cards.length+'</b> card'+(cards.length===1?'':'s')+', the rafters and <b>'+fmtAbbr(wallet)+' CRED</b> into this browser.');
 }
 // ---- CARD PACKS -----------------------------------------------------------
 // Pricey CRED buys a chance at exotic cards that exist nowhere on the floor:
 // pulled, not earned. The top rarities are brutally hard, so a pulled LEGEND or
 // ASCENDED is a real flex. Two tiers; PRIME shifts the odds toward the broken stuff.
 // Tiers escalate from pocket change to whale-only. Because a single legend can be
 // worth tens of billions, the high packs cost billions to trillions or ripping
 // them would be meaningless. Higher tiers floor the rarity, scale the stat line
 // (vmul) and lean gem-heavy, and mint a distinct SERIES so the cards feel unique.
 var PACKS={
  std:{cost:250000,name:'STANDARD',series:'STREET',house:'THE STALL',vmul:1,odds:[['common',96],['rare',3.6],['legend',0.35],['ascended',0.05]],fin:[['',90],['shiny',8],['gold',1.5],['emerald',0.4],['sapphire',0.08],['diamond',0.02]]},
  prime:{cost:1000000,name:'PRIME',series:'PRIME',house:'THE VAULT',vmul:1.4,odds:[['common',86],['rare',12],['legend',1.8],['ascended',0.2]],fin:[['',80],['shiny',14],['gold',4.5],['emerald',1.2],['sapphire',0.25],['diamond',0.05]]},
  vault:{cost:50000000,name:'VAULT',series:'RESERVE',house:'THE RESERVE',vmul:2.2,odds:[['rare',88],['legend',11],['ascended',1]],fin:[['',60],['shiny',24],['gold',11],['emerald',4],['sapphire',0.8],['diamond',0.2]]},
  eclipse:{cost:1000000000,name:'ECLIPSE',series:'ECLIPSE',house:'ECLIPSE LINE',vmul:4,odds:[['legend',97],['ascended',3]],fin:[['',35],['shiny',30],['gold',22],['emerald',9],['sapphire',3],['diamond',1]]},
  cosmic:{cost:50000000000,name:'COSMIC',series:'COSMIC',house:'COSMIC SERIES',vmul:10,odds:[['legend',88],['ascended',12]],fin:[['shiny',28],['gold',34],['emerald',24],['sapphire',11],['diamond',3]]},
  singularity:{cost:1000000000000,name:'SINGULARITY',series:'SINGULARITY',house:'SINGULARITY',vmul:1,odds:[['ascended',100]],fin:[['gold',40],['emerald',36],['sapphire',20],['diamond',4]]}
 };
 var PACK_ORDER=['std','prime','vault','eclipse','cosmic','singularity'];
 function rollW(list){var t=0,i;for(i=0;i<list.length;i++)t+=list[i][1];var x=rnd()*t,a=0;for(i=0;i<list.length;i++){a+=list[i][1];if(x<a)return list[i][0];}return list[0][0];}
 var EXA=['PRISM','OBLIVION','SERAPH','VANTA','CHRONO','NOVA','WRAITH','ZENITH','OMEGA','PHANTOM','RADIANT','ECLIPSE','TITAN','MIRAGE','QUASAR','SPECTER','AETHER','HALO','VESPER','NULLION','CRYO','PYRE','OBELISK','SABLE'];
 var EXB=['SOVEREIGN','CROWN','SIGIL','GENESIS','RELIC','ORACLE','THRONE','APEX','MYTH','REIGN','FLUX','ASCENT','PARADOX','ETERNA','ZERO','INFINITE','RIFT','HALON','PARAGON','WARDEN'];
 function exoticName(){var r=mulberry(hashStr('ex-'+Math.floor(rnd()*1e9)+'-'+cards.length));return EXA[(r()*EXA.length)|0]+'-'+EXB[(r()*EXB.length)|0]+'-'+('0'+((r()*256)|0).toString(16).toUpperCase()).slice(-2);}
 function mintCard(tier){
  var pk=PACKS[tier]||PACKS.std,vm=pk.vmul||1,rar=rollW(pk.odds),fin=rollW(pk.fin),risk=rnd(),peak,res;
  if(rar==='ascended'){peak=1e13+rnd()*9e13;res={legend:true,podium:true,ascended:true};}
  else if(rar==='legend'){peak=(1e9+rnd()*6e10)*vm;res={legend:true,podium:true,ascended:false};}
  else if(rar==='rare'){peak=(1e6+rnd()*1.2e7)*vm;res={legend:false,podium:true,ascended:false};}
  else{peak=(1500+rnd()*12000)*vm;res={legend:false,podium:false,ascended:false};}
  res.season=season;res.rank=(rar==='common'?(4+(rnd()*8|0)):1);res.finalBank=Math.round(peak*(0.3+rnd()*0.5));res.peak=Math.round(peak);res.maxStreak=(rar==='ascended'?20:(rar==='legend'?12:(rar==='rare'?7:3)))+(rnd()*8|0);res.biggestWin=Math.round(peak*(0.2+rnd()*0.4));res.fromPack=tier;
  var label=(rar==='ascended'?'ASCENDED':rar.toUpperCase());
  return {code:newCode(900+(rnd()*90|0)),id:-1-((rnd()*1e6)|0),name:exoticName(),house:pk.house||'THE VAULT',series:pk.series||'',fade:(rnd()<0.5),risk:risk,sel:0.5,press:0.5,finish:fin,origin:'pack',packTier:tier,rarPack:rar,claimedSeason:season,resolved:res,historic:pk.name+' SERIES // '+label+' // peak '+fmtAbbr(peak)+(fin?(' // '+FIN[fin].n):'')};
 }
 var packBusy=false;
 function closePack(){var m=document.getElementById('packmodal');if(m)m.className='';packBusy=false;}
 function openPack(tier){
  if(packBusy)return; // one rip at a time: blocks Enter-mashing / double-open
  var pk=PACKS[tier];if(!pk)return;
  if(wallet<pk.cost){toast('Not enough CRED. A '+pk.name+' pack is '+fmt(pk.cost)+' CRED.');return;}
  packBusy=true;
  if(!window.confirm('Rip a '+pk.name+' pack for '+fmt(pk.cost)+' CRED? The pull is yours whatever it is.')){packBusy=false;return;}
  wallet-=pk.cost;var c=mintCard(tier);cards.unshift(c);cards=cards.slice(0,60);
  pushPull('YOU',c.rarPack,c.finish,(ascended(c)?1e15:cardValue(c)),true);
  save();renderCollection();renderAll();showReveal(c,tier);
 }
 function drawPackBar(){var el=document.getElementById('packbtns');if(!el)return;el.innerHTML=PACK_ORDER.map(function(k){var p=PACKS[k];return '<button class="listen packbtn" data-pk="'+k+'" type="button">[ '+p.name+' // '+fmtAbbr(p.cost)+' ]</button>';}).join('');}
 function showReveal(c,tier){
  var modal=document.getElementById('packmodal');if(!modal){return;}var pk=PACKS[tier]||PACKS.std,rar=c.rarPack;
  var col=rar==='ascended'?'#d8f0ff':(rar==='legend'?'#ffd56b':(rar==='rare'?'#6fb8ff':'#9ac46a'));
  var label=(rar==='ascended'?'ASCENDED':rar.toUpperCase())+(c.finish?(' '+FIN[c.finish].n):'');
  modal.className='on rip '+rar;
  modal.innerHTML='<div class="pkstate" id="pkstate">RIPPING '+pk.name+' PACK</div><div class="pkglyph">[ ? ]</div>';
  setTimeout(function(){
   modal.className='on reveal '+rar;
   modal.innerHTML='<div class="pkstate">'+pk.name+' PACK PULL</div><div class="pkrar" style="color:'+col+'">'+label+'</div><div class="pkcard"><canvas id="pkcanvas" width="320" height="170"></canvas><div style="font-size:18px;font-weight:700;color:#e7e2d4;margin-top:8px">'+esc(c.name)+'</div><div style="font-size:12px;color:#a9a596;margin-top:3px">'+esc(c.house)+' // worth '+(ascended(c)?'INFINITE CRED':fmtAbbr(cardValue(c))+' CRED')+'</div></div><div class="pkbtns"><button class="listen" id="pkdone" type="button">[ INTO THE COLLECTION ]</button><button class="listen" id="pkshare" type="button">[ SHARE ]</button></div>';
   var cv=document.getElementById('pkcanvas');if(cv)drawEmblem(cv,c,rarOf(c));
   var d=document.getElementById('pkdone');if(d)d.onclick=function(){closePack();};
   var sh=document.getElementById('pkshare');if(sh)sh.onclick=function(){shareCard(c.code);};
  },1300);
 }
 function toast(m){var el=document.getElementById('floorresolve');if(!el)return;el.style.display='block';el.innerHTML=esc(m);setTimeout(function(){el.style.display='none';},4200);}
 // ---- THE COLLECTORS (resident floor NPCs) ---------------------------------
 // No backend and no other humans, so the floor is populated by resident collector
 // bots: they rip packs, browse and chatter in a live feed, and make YOU real
 // offers (cash or trade) on your cards. Accepting moves CRED/cards atomically.
 var BOTS=['ZephyrTrades','NocturneByte','VoidHandler','NeonBaron','SableGhost','OrbitalKid','PrismFiend','HexCollector','LumenDealer','QuietRiot','AceOfVoid','GrailHunter','ByteWraith','EchoDealer','RiftRunner','GildedFox','NullSeeker','VantaWhale','OmenTrader','CinderKid','SaintVex','HollowApe','DeltaMyth','KiloCrown','MidasByte','TheArchivist','GhostOfTheVault','SolomonGrip','EmberQueen','ZeroCoolXX','VelvetRuin','OnyxBaron','StaticSaint','LunaVoid','CovenantX','TungstenJoe','ApexDegen','MoonlitFax','CardSharkAI','WraithMother','ObsidianOx','GammaWolf','HelixHazard','PaperHandsNo','DiamondMindy','TheCustodian','VaultJackal','NeuralNoir','QuantumFox','BlackSiteBet','CrownVulture','SilentLedger','TitaniumTia','RogueOracle','EchoChamberX','BezosOfBets','TheLiquidator','GoldfingerAI','MammonPrime','HouseAlwaysX'];
 function botName(){return BOTS[(rnd()*BOTS.length)|0];}
 // ---- LEADERBOARDS (you vs the floor) --------------------------------------
 // Two ladders: NET WORTH (your liquid CRED plus the value of every card you hold)
 // and RAREST PULLS. The whale bots seed both with absurd numbers so there is
 // always someone above you to chase. Their net worths drift up over the session.
 var WHALES=BOTS.slice(0),botNW={},nwSeeded=false,boardPulls=[],NWCAP=1e22;
 // Heavy-tailed fortunes: most collectors sit in the millions-to-trillions, but the
 // distribution has a fat tail so a handful are absurd (quadrillions, quintillions,
 // even sextillions). With ~60 of them, there is always a monster above you.
 function seedNW(name){var r=mulberry(hashStr('nw|'+name));
  var tiers=[[1e8,16],[1e9,18],[1e10,16],[1e11,12],[1e12,10],[1e13,8],[1e14,6],[1e15,4],[5e15,3],[1e17,2],[1e18,1.2],[1e20,0.5],[1e21,0.2]];
  var t=0,i;for(i=0;i<tiers.length;i++)t+=tiers[i][1];var x=r()*t,a=0,base=1e9;for(i=0;i<tiers.length;i++){a+=tiers[i][1];if(x<a){base=tiers[i][0];break;}}
  return Math.round(base*(0.4+r()*4.6));}
 // A few flagship whales are hard-anchored into the quintillions so the very top of
 // the board is always cartoonishly rich, no matter how the seed falls.
 var MEGA={'HouseAlwaysX':9.2e20,'MammonPrime':4.4e20,'BezosOfBets':8.0e19,'TheLiquidator':2.6e19,'GoldfingerAI':6.1e18,'TheCustodian':1.4e18};
 function ensureNW(){if(nwSeeded)return;for(var i=0;i<WHALES.length;i++)botNW[WHALES[i]]=MEGA[WHALES[i]]||seedNW(WHALES[i]);nwSeeded=true;}
 function myNetWorth(){var s=wallet,i;for(i=0;i<cards.length;i++)s+=Math.min(cardValue(cards[i]),1e15);return s;}
 function pullValue(rar,fin,vm){if(rar==='ascended')return 1e15;var m=finMult(fin),v;if(rar==='legend')v=(1e9+rnd()*6e10)*(vm||1);else if(rar==='rare')v=(1e6+rnd()*1.2e7)*(vm||1);else v=(1500+rnd()*12000)*(vm||1);return Math.round(v*m);}
 function rarColor(r){return r==='ascended'?'#d8f0ff':(r==='legend'?'#ffd56b':(r==='rare'?'#6fb8ff':'#9ac46a'));}
 function pushPull(who,rar,fin,val,you){boardPulls.push({who:who,rar:rar,fin:fin,val:val,you:!!you});boardPulls.sort(function(a,b){return b.val-a.val;});boardPulls=boardPulls.slice(0,40);drawPulls();}
 function seedBoard(){ensureNW();if(boardPulls.length)return;for(var i=0;i<10;i++){var b=WHALES[(rnd()*WHALES.length)|0],rar=rollW([['legend',62],['ascended',38]]),fin=rollW([['gold',28],['emerald',26],['sapphire',26],['diamond',20]]);pushPull(b,rar,fin,pullValue(rar,fin,rar==='ascended'?1:7),false);}}
 function drawPulls(){var el=document.getElementById('pullboard');if(!el)return;var list=boardPulls.slice(0,12);el.innerHTML=list.map(function(p,i){var v=p.val>=1e15?'&infin;':fmtAbbr(p.val);return '<div class="lbrow'+(p.you?' you':'')+'"><span class="lr">'+(i+1)+'</span><span class="ln">'+esc(p.who)+'</span><span class="lt" style="color:'+rarColor(p.rar)+'">'+(p.rar==='ascended'?'ASCENDED':p.rar.toUpperCase())+(p.fin&&FIN[p.fin]?(' '+FIN[p.fin].n):'')+'</span><span class="lv">'+v+'</span></div>';}).join('')||'<span class="sub">no pulls yet</span>';}
 function drawNW(){var el=document.getElementById('nwboard');if(!el)return;ensureNW();var rows=[],i;for(i=0;i<WHALES.length;i++)rows.push({name:WHALES[i],nw:botNW[WHALES[i]]||0});var me=myNetWorth();rows.push({name:'YOU',nw:me,you:true});rows.sort(function(a,b){return b.nw-a.nw;});var myRank=0;for(i=0;i<rows.length;i++)if(rows[i].you){myRank=i+1;break;}var html=rows.slice(0,12).map(function(r,i){return '<div class="lbrow'+(r.you?' you':'')+'"><span class="lr">'+(i+1)+'</span><span class="ln">'+esc(r.name)+'</span><span class="lv">'+fmtAbbr(r.nw)+' CRED</span></div>';}).join('');if(myRank>12)html+='<div class="lbrow you"><span class="lr">'+myRank+'</span><span class="ln">YOU</span><span class="lv">'+fmtAbbr(me)+' CRED</span></div>';el.innerHTML=html;}
 function drawBoards(){drawNW();drawPulls();}
 function nwDrift(){ensureNW();var n=2+(rnd()*3|0),i,b;
  for(b in CURSES){if(cursed(b))botNW[b]=Math.max(1e6,(botNW[b]||seedNW(b))*(0.88+rnd()*0.07));} // the cursed bleed
  for(i=0;i<n;i++){b=WHALES[(rnd()*WHALES.length)|0];if(cursed(b))continue;botNW[b]=Math.min(NWCAP,(botNW[b]||seedNW(b))*(1+rnd()*0.03));}drawNW();}
 // ---- THE CARD MARKET ------------------------------------------------------
 // Each rarity sector is an index that random-walks and reverts toward 1.0, so
 // card values rise and fall over time like a real (small, weird) economy.
 function tickMarket(){var k=['common','rare','legend'],i;for(i=0;i<k.length;i++){var m=MARKET[k[i]];m=m+(1-m)*0.03+(rnd()-0.5)*0.13;MARKET[k[i]]=Math.max(0.35,Math.min(3,m));}drawMarket();drawBoards();save();}
 function drawMarket(){var el=document.getElementById('marketbar');if(!el)return;function cell(lbl,m){var pct=Math.round((m-1)*100),col=pct>=0?'#6ee07a':'#ff8c7d';return '<span style="margin-right:16px">'+lbl+' <b style="color:'+col+'">'+(pct>=0?'+':'')+pct+'%</b></span>';}el.innerHTML='<span style="color:#8d8a7c;letter-spacing:.1em;margin-right:8px">CARD MARKET</span>'+cell('COMMON',MARKET.common)+cell('RARE',MARKET.rare)+cell('LEGEND',MARKET.legend);}
 // ---- CURSES ---------------------------------------------------------------
 // Lowballed? Hex the collector. A curse instantly knocks down their net worth and
 // gives them terrible luck (bleeding money, whiffing every pack) for two minutes.
 function nowMs(){return Date.now();}
 function cursed(name){return (CURSES[name]||0)>nowMs();}
 function curseBot(name){ensureNW();CURSES[name]=nowMs()+120000;botNW[name]=Math.max(1e6,(botNW[name]||seedNW(name))*0.55);feedPush('<b style="color:#c77dff">You laid a CURSE on '+esc(name)+'. Terrible luck follows them for two minutes.</b>');drawNW();}
 // ---- THE GLOBAL BOARD (cross-browser, server-free) ------------------------
 // You post a score by opening a prefilled GitHub issue; the daily build harvests
 // every score issue into a static leaderboard.json the site reads. No backend.
 var REPO=(window.MKT_REPO||'Mattbusel/tech-oracle');
 function getHandle(){try{return localStorage.getItem('signal_floor_handle')||'';}catch(e){return '';}}
 function setHandle(h){try{localStorage.setItem('signal_floor_handle',h);}catch(e){}}
 function bestPull(){var top=null,i;for(i=0;i<boardPulls.length;i++){var p=boardPulls[i];if(p.you&&(!top||p.val>top.val))top=p;}
  for(i=0;i<cards.length;i++){var c=cards[i],v=ascended(c)?1e15:cardValue(c);if(!top||v>top.val)top={rar:tierOf(c).toLowerCase(),fin:c.finish,val:v};}return top;}
 function postScore(){
  var h=getHandle();
  if(!h){h=(window.prompt('Pick a handle for the GLOBAL board (letters and numbers):','')||'').replace(/[^A-Za-z0-9_-]/g,'').slice(0,24).toUpperCase();if(!h)return;setHandle(h);}
  var nw=Math.round(myNetWorth()),bp=bestPull();
  var pull=bp?((bp.rar?bp.rar.toUpperCase():'CARD')+(bp.fin&&FIN[bp.fin]?('-'+FIN[bp.fin].n):'')):'NONE',pv=bp?Math.round(bp.val):0;
  var line='SIGNAL-FLOOR-SCORE v=1 handle='+h+' nw='+nw+' pull='+pull+' pullval='+pv;
  var title='Floor score: '+h+' // '+fmtAbbr(nw)+' CRED';
  var body='My net worth on THE SIGNAL floor.\n\n'+line+'\n\nLeave the line above intact so the board can read it.\nThe board: '+location.origin+location.pathname;
  var url='https://github.com/'+REPO+'/issues/new?labels=score&title='+encodeURIComponent(title)+'&body='+encodeURIComponent(body);
  window.open(url,'_blank','noopener');
  toast('Opening GitHub to post your score. It joins the global board on the next daily build.');
 }
 function drawGlobal(d){var nb=document.getElementById('gnwboard'),pb=document.getElementById('gpullboard'),nw=(d&&d.networth)||[],pl=(d&&d.pulls)||[];
  if(nb)nb.innerHTML=nw.length?nw.slice(0,12).map(function(r,i){return '<div class="lbrow"><span class="lr">'+(i+1)+'</span><span class="ln">'+esc(r.handle)+'</span><span class="lv">'+fmtAbbr(r.nw)+' CRED</span></div>';}).join(''):'<span class="sub">no scores posted yet. be the first to POST MY SCORE.</span>';
  if(pb)pb.innerHTML=pl.length?pl.slice(0,12).map(function(r,i){var v=r.pullval>=1e15?'&infin;':fmtAbbr(r.pullval);return '<div class="lbrow"><span class="lr">'+(i+1)+'</span><span class="ln">'+esc(r.handle)+'</span><span class="lt">'+esc(r.pull||'')+'</span><span class="lv">'+v+'</span></div>';}).join(''):'<span class="sub">no pulls posted yet.</span>';}
 function fetchGlobal(){fetch('api/leaderboard.json',{cache:'no-store'}).then(function(r){return r.ok?r.json():null;}).then(function(d){drawGlobal(d);}).catch(function(){drawGlobal(null);});}
 var feedItems=[];
 function feedPush(html){feedItems.unshift('<div style="padding:5px 0;border-bottom:1px dotted #2a2c28">'+html+'</div>');feedItems=feedItems.slice(0,28);var el=document.getElementById('collfeed');if(el)el.innerHTML=feedItems.join('');}
 // The collectors are unhinged whales: they rip the expensive tiers, so big pulls
 // keep hitting the leaderboard and their net worths stay absurd.
 function botPackPick(){return rollW([['std',26],['prime',24],['vault',20],['eclipse',16],['cosmic',10],['singularity',4]]);}
 function botRip(){ensureNW();var tier=botPackPick(),pk=PACKS[tier],b=botName();
  if(cursed(b)){botNW[b]=Math.max(1e6,(botNW[b]||seedNW(b))*0.96);feedPush('<span style="color:#c77dff">'+esc(b)+' burned a '+pk.name+' pack and pulled NOTHING. The curse holds.</span>');drawNW();return;}
  var rar=rollW(pk.odds),fin=rollW(pk.fin),val=pullValue(rar,fin,pk.vmul||1);
  botNW[b]=Math.min(NWCAP,(botNW[b]||seedNW(b))*(rar==='ascended'?1.05:(rar==='legend'?1.02:1.004)));
  if(rar==='ascended')feedPush('<b style="color:#d8f0ff">'+esc(b)+' RIPPED A '+pk.name+' PACK AND PULLED AN ASCENDED ONE-OF-ONE'+(fin?(' '+FIN[fin].n):'')+'. The floor is losing it.</b>');
  else if(rar==='legend')feedPush('<b style="color:#ffd56b">'+esc(b)+'</b> ripped a '+pk.name+' pack and hit a <b style="color:#ffd56b">LEGEND</b>'+(fin?(' '+FIN[fin].n):'')+'.');
  else if(rar==='rare')feedPush('<b>'+esc(b)+'</b> ripped a '+pk.name+' pack: a RARE.');
  else feedPush('<span style="color:#8d8a7c">'+esc(b)+' ripped a '+pk.name+' pack. nothing wild.</span>');
  if(rar==='legend'||rar==='ascended')pushPull(b,rar,fin,val,false);
  drawNW();}
 function botChatter(){var b=botName(),c=cards.length?cards[(rnd()*cards.length)|0]:null,m=[];
  m.push(esc(botName())+' and '+esc(botName())+' just traded cards.');
  m.push(esc(b)+' is hunting for a diamond finish.');
  m.push(esc(b)+' listed a card on the floor.');
  if(c){m.push(esc(b)+' is eyeing your <b>'+esc(c.name)+'</b>.');m.push(esc(b)+' asked what you would take for <b>'+esc(c.name)+'</b>.');}
  feedPush('<span style="color:#a9a596">'+m[(rnd()*m.length)|0]+'</span>');}
 function feedTick(){var cu=[],k;for(k in CURSES)if(cursed(k))cu.push(k);if(cu.length&&rnd()<0.4){var b=cu[(rnd()*cu.length)|0],m=['stumbled and lost a card','got fleeced at the table','watched a sure bet collapse','keeps drawing dead','missed on everything'];feedPush('<span style="color:#c77dff">'+esc(b)+' '+m[(rnd()*m.length)|0]+'. (cursed)</span>');return;}if(rnd()<0.58)botRip();else botChatter();}
 var curOffer=null;
 function makeOffer(){
  if(curOffer||!cards.length)return;
  var c=cards[(rnd()*cards.length)|0],v=cardValue(c),b=botName(),trade=rnd()<0.35;
  if(trade){var give=mintCard(rnd()<0.2?'prime':'std');give.origin='bot';give.house='THE FLOOR';curOffer={type:'trade',bot:b,yourCode:c.code,give:give};
   showOffer('<b>'+esc(b)+'</b> wants to trade their <b>'+esc(give.name)+'</b> ('+(give.rarPack||'card').toUpperCase()+(give.finish?(' '+FIN[give.finish].n):'')+', ~'+(ascended(give)?'INFINITE':fmtAbbr(cardValue(give)))+' CRED) for your <b>'+esc(c.name)+'</b> (~'+(ascended(c)?'INFINITE':fmtAbbr(v))+').');}
  else{var mult=0.7+rnd()*0.9,amt=Math.max(100,Math.round(Math.min(v,1e14)*mult));curOffer={type:'buy',bot:b,yourCode:c.code,amt:amt};
   var tone=mult>1.25?' <span style="color:#6ee07a">(over market!)</span>':(mult<0.85?' <span style="color:#ff8c7d">(lowball)</span>':'');
   showOffer('<b>'+esc(b)+'</b> offers <b>'+fmt(amt)+' CRED</b> for your <b>'+esc(c.name)+'</b> (worth ~'+(ascended(c)?'INFINITE':fmtAbbr(v))+')'+tone+'.');}
 }
 function showOffer(html){var el=document.getElementById('offerbox');if(!el)return;el.style.display='block';el.innerHTML='<div style="margin-bottom:7px">'+html+'</div><div style="display:flex;gap:6px;flex-wrap:wrap"><button id="offyes" class="tbtn" type="button">[ ACCEPT ]</button><button id="offno" class="tbtn" type="button">[ DECLINE ]</button><button id="offcurse" class="tbtn" type="button" style="border-color:#7a4ea8;color:#d8b6ff">[ CURSE THEM ]</button></div>';var y=document.getElementById('offyes'),n=document.getElementById('offno'),cu=document.getElementById('offcurse');if(y)y.onclick=acceptOffer;if(n)n.onclick=declineOffer;if(cu)cu.onclick=curseFromOffer;}
 function curseFromOffer(){if(!curOffer)return;var b=curOffer.bot;curOffer=null;hideOffer();curseBot(b);}
 function hideOffer(){var el=document.getElementById('offerbox');if(el){el.style.display='none';el.innerHTML='';}}
 function acceptOffer(){if(!curOffer)return;var o=curOffer,idx=-1,i;for(i=0;i<cards.length;i++)if(cards[i].code===o.yourCode)idx=i;if(idx<0){declineOffer();return;}var mine=cards[idx];
  if(o.type==='buy'){cards.splice(idx,1);wallet+=o.amt;feedPush('<b style="color:#6ee07a">You sold '+esc(mine.name)+' to '+esc(o.bot)+' for '+fmt(o.amt)+' CRED.</b>');}
  else{cards.splice(idx,1);o.give.code=newCode(800+(rnd()*99|0));cards.unshift(o.give);cards=cards.slice(0,60);feedPush('<b style="color:#6ee07a">You traded '+esc(mine.name)+' to '+esc(o.bot)+' for '+esc(o.give.name)+'.</b>');}
  curOffer=null;hideOffer();save();renderCollection();renderAll();}
 function declineOffer(){if(curOffer)feedPush('<span style="color:#8d8a7c">You passed on '+esc(curOffer.bot)+'.</span>');curOffer=null;hideOffer();}
 function offerTick(){if(!curOffer&&cards.length&&rnd()<0.7)makeOffer();}
 // Procedural emblem: a unique seeded crest per card, colored by its genes.
 function hashStr(s){var h=2166136261>>>0;s=s||'';for(var i=0;i<s.length;i++){h^=s.charCodeAt(i);h=Math.imul(h,16777619);}return h>>>0;}
 function mulberry(a){return function(){a|=0;a=a+0x6D2B79F5|0;var t=Math.imul(a^a>>>15,1|a);t=t+Math.imul(t^t>>>7,61|t)^t;return((t^t>>>14)>>>0)/4294967296;};}
 // ---- CARD ART ENGINE -------------------------------------------------------
 // Each card draws one of several generative SYSTEMS picked from its seed, so two
 // cards look fundamentally different, not merely recolored. Rarity adds glow and
 // metal; gem finishes add real foil sheen; ASCENDED cards are deliberately
 // corrupted (channel-shift datamosh + scanlines + tears) so the rarest, most
 // broken cards actually LOOK broken. All deterministic from the card code.
 function hsl(h,s,l){return 'hsl('+(((h%360)+360)%360)+','+s+'%,'+l+'%)';}
 function artPalette(card,rarity,rng){
  var hue=Math.floor((card.risk!=null?card.risk:rng())*360),acc=(hue+90+Math.floor(rng()*150))%360;
  var p={bg0:hsl(hue,40,9),bg1:'#06080b',ink:hsl(acc,72,64),ink2:hsl(hue,58,52),glow:0,metal:null};
  if(rarity==='rare'){p.bg0=hsl(hue,52,12);p.ink=hsl(acc,84,67);p.glow=8;}
  if(rarity==='legend'){p.bg0='#2a1e05';p.bg1='#0a0a08';p.ink='#ffd56b';p.ink2='#caa64a';p.glow=12;p.metal='gold';}
  if(card.finish&&FIN[card.finish]){p.ink=FIN[card.finish].c;p.ink2=FIN[card.finish].c;p.glow=Math.max(p.glow,9);}
  return p;
 }
 function artBg(ctx,W,H,p,rng){
  var g=ctx.createRadialGradient(W*0.5,H*0.42,2,W*0.5,H*0.5,Math.max(W,H)*0.78);
  g.addColorStop(0,p.bg0);g.addColorStop(1,p.bg1);ctx.fillStyle=g;ctx.fillRect(0,0,W,H);
  if(p.metal==='gold'){var go=ctx.createRadialGradient(W/2,H*0.46,1,W/2,H*0.46,Math.max(W,H)*0.5);go.addColorStop(0,'rgba(255,213,107,0.18)');go.addColorStop(1,'rgba(255,213,107,0)');ctx.fillStyle=go;ctx.fillRect(0,0,W,H);}
  ctx.fillStyle='rgba(255,255,255,0.05)';for(var i=0;i<W*H/900;i++)ctx.fillRect((rng()*W)|0,(rng()*H)|0,1,1);
  var v=ctx.createRadialGradient(W/2,H/2,Math.min(W,H)*0.18,W/2,H/2,Math.max(W,H)*0.72);
  v.addColorStop(0,'rgba(0,0,0,0)');v.addColorStop(1,'rgba(0,0,0,0.55)');ctx.fillStyle=v;ctx.fillRect(0,0,W,H);
 }
 function sysOrbital(ctx,W,H,p,rng){var cx=W/2,cy=H/2,R=Math.min(W,H)*0.46,rings=2+(rng()*3|0),lw=Math.max(0.8,W/200);ctx.lineWidth=lw;
  for(var r=0;r<rings;r++){var rr=R*(0.32+0.68*(r+1)/rings),sq=0.5+rng()*0.5,rot=rng()*3;ctx.strokeStyle=p.ink2;ctx.globalAlpha=0.3;ctx.beginPath();ctx.ellipse(cx,cy,rr,rr*sq,rot,0,7);ctx.stroke();var bodies=1+(rng()*3|0);for(var b=0;b<bodies;b++){var a=rng()*7,x=cx+Math.cos(a)*rr*Math.cos(rot)-Math.sin(a)*rr*sq*Math.sin(rot),y=cy+Math.cos(a)*rr*Math.sin(rot)+Math.sin(a)*rr*sq*Math.cos(rot);ctx.globalAlpha=0.92;ctx.fillStyle=p.ink;ctx.shadowColor=p.ink;ctx.shadowBlur=p.glow;ctx.beginPath();ctx.arc(x,y,lw*(1.4+rng()*2),0,7);ctx.fill();}}
  ctx.shadowBlur=0;ctx.globalAlpha=1;ctx.fillStyle=p.ink;ctx.beginPath();ctx.arc(cx,cy,lw*2.3,0,7);ctx.fill();}
 function sysMandala(ctx,W,H,p,rng){var cx=W/2,cy=H/2,R=Math.min(W,H)*0.47,k=5+(rng()*8|0),arms=2+(rng()*3|0),lw=Math.max(0.8,W/230),i;ctx.lineWidth=lw;ctx.strokeStyle=p.ink;ctx.fillStyle=p.ink;ctx.shadowColor=p.ink;ctx.shadowBlur=p.glow*0.6;
  var pts=[];for(i=0;i<arms;i++)pts.push([R*(0.25+rng()*0.75),(rng()-0.5)*0.6,lw*(1+rng()*2)]);
  for(var s=0;s<k;s++){ctx.save();ctx.translate(cx,cy);ctx.rotate(s/k*Math.PI*2);ctx.globalAlpha=0.85;ctx.beginPath();ctx.moveTo(0,0);for(i=0;i<pts.length;i++)ctx.lineTo(Math.cos(pts[i][1])*pts[i][0],Math.sin(pts[i][1])*pts[i][0]);ctx.stroke();for(i=0;i<pts.length;i++){ctx.beginPath();ctx.arc(Math.cos(pts[i][1])*pts[i][0],Math.sin(pts[i][1])*pts[i][0],pts[i][2],0,7);ctx.fill();}ctx.restore();}ctx.shadowBlur=0;ctx.globalAlpha=1;}
 function sysConstellation(ctx,W,H,p,rng){var n=6+(rng()*10|0),ns=[],i,j,lw=Math.max(0.7,W/240);for(i=0;i<n;i++)ns.push([W*0.12+rng()*W*0.76,H*0.12+rng()*H*0.76,lw*(1+rng()*2.2)]);
  ctx.strokeStyle=p.ink2;ctx.globalAlpha=0.25;ctx.lineWidth=lw*0.7;var d2=(W*0.34)*(W*0.34);for(i=0;i<n;i++)for(j=i+1;j<n;j++){var dx=ns[i][0]-ns[j][0],dy=ns[i][1]-ns[j][1];if(dx*dx+dy*dy<d2){ctx.beginPath();ctx.moveTo(ns[i][0],ns[i][1]);ctx.lineTo(ns[j][0],ns[j][1]);ctx.stroke();}}
  ctx.globalAlpha=1;ctx.fillStyle=p.ink;ctx.shadowColor=p.ink;ctx.shadowBlur=p.glow;for(i=0;i<n;i++){ctx.beginPath();ctx.arc(ns[i][0],ns[i][1],ns[i][2],0,7);ctx.fill();}ctx.shadowBlur=0;}
 function sysCircuit(ctx,W,H,p,rng){var step=Math.max(10,W/9),lw=Math.max(0.8,W/210),x,y;ctx.strokeStyle=p.ink2;ctx.lineWidth=lw;ctx.globalAlpha=0.5;
  for(x=step;x<W;x+=step){var turn=step+((rng()*Math.max(1,(H/step)|0))|0)*step;ctx.beginPath();ctx.moveTo(x,0);ctx.lineTo(x,turn);ctx.lineTo(x+(rng()<0.5?step:-step),turn);ctx.stroke();}
  ctx.globalAlpha=0.95;ctx.fillStyle=p.ink;ctx.shadowColor=p.ink;ctx.shadowBlur=p.glow*0.6;for(x=step;x<W;x+=step)for(y=step;y<H;y+=step)if(rng()<0.16){ctx.beginPath();ctx.arc(x,y,lw*1.7,0,7);ctx.fill();}ctx.shadowBlur=0;ctx.globalAlpha=1;}
 function sysWave(ctx,W,H,p,rng){var cx=W*(0.3+rng()*0.4),cy=H*(0.3+rng()*0.4),rings=5+(rng()*8|0),gap=Math.min(W,H)/(rings*1.5),lw=Math.max(0.7,W/260),i;ctx.lineWidth=lw;
  ctx.strokeStyle=p.ink;for(i=1;i<=rings;i++){ctx.globalAlpha=0.5*(1-i/rings)+0.12;ctx.beginPath();ctx.arc(cx,cy,i*gap,0,7);ctx.stroke();}
  var cx2=W-cx;ctx.strokeStyle=p.ink2;for(i=1;i<=rings;i++){ctx.globalAlpha=0.4*(1-i/rings)+0.1;ctx.beginPath();ctx.arc(cx2,cy,i*gap,0,7);ctx.stroke();}ctx.globalAlpha=1;}
 function sysShards(ctx,W,H,p,rng){var cx=W/2,cy=H/2,R=Math.min(W,H)*0.52,n=4+(rng()*6|0),lw=Math.max(0.8,W/220),i,ang=[];for(i=0;i<n;i++)ang.push(rng()*Math.PI*2);ang.sort(function(a,b){return a-b;});
  for(i=0;i<n;i++){var a0=ang[i],a1=(i+1<n?ang[i+1]:ang[0]+Math.PI*2),r0=R*(0.4+rng()*0.6),r1=R*(0.4+rng()*0.6);ctx.beginPath();ctx.moveTo(cx,cy);ctx.lineTo(cx+Math.cos(a0)*r0,cy+Math.sin(a0)*r0);ctx.lineTo(cx+Math.cos(a1)*r1,cy+Math.sin(a1)*r1);ctx.closePath();ctx.globalAlpha=0.16+rng()*0.24;ctx.fillStyle=(i%2?p.ink:p.ink2);ctx.fill();ctx.globalAlpha=0.6;ctx.lineWidth=lw*0.8;ctx.strokeStyle=p.ink;ctx.stroke();}
  ctx.globalAlpha=1;ctx.fillStyle=p.ink;ctx.shadowColor=p.ink;ctx.shadowBlur=p.glow;ctx.beginPath();ctx.arc(cx,cy,lw*1.8,0,7);ctx.fill();ctx.shadowBlur=0;}
 function foil(ctx,W,H,str,rng){ctx.save();ctx.globalCompositeOperation='lighter';
  var g=ctx.createLinearGradient(0,0,W,H);g.addColorStop(0,'#ff4d6d');g.addColorStop(0.25,'#ffd56b');g.addColorStop(0.5,'#4fe0a0');g.addColorStop(0.75,'#5fb8ff');g.addColorStop(1,'#c77dff');
  ctx.globalAlpha=0.05+0.10*str;ctx.fillStyle=g;ctx.fillRect(0,0,W,H);
  var off=0.2+rng()*0.6,s=ctx.createLinearGradient(0,0,W,H);s.addColorStop(Math.max(0,off-0.14),'rgba(255,255,255,0)');s.addColorStop(off,'rgba(255,255,255,0.55)');s.addColorStop(Math.min(1,off+0.14),'rgba(255,255,255,0)');
  ctx.globalAlpha=0.12+0.22*str;ctx.fillStyle=s;ctx.fillRect(0,0,W,H);ctx.restore();}
 function glitch(ctx,W,H,rng){try{var slabs=4+(rng()*5|0);for(var i=0;i<slabs;i++){var sy=(rng()*H)|0,sh=Math.max(2,(rng()*H*0.16)|0);if(sy+sh>H)sh=H-sy;if(sh<1)continue;var dx=((rng()-0.5)*W*0.22)|0;var img=ctx.getImageData(0,sy,W,sh);ctx.putImageData(img,dx,sy);}}catch(e){}
  ctx.save();for(var t=0;t<3;t++){var ty=(rng()*H)|0;ctx.globalAlpha=0.5;ctx.fillStyle=['#ff2d55','#5fb8ff','#4fe0a0'][t%3];ctx.fillRect(0,ty,W,1+(rng()*2|0));}
  ctx.globalAlpha=0.12;ctx.fillStyle='#000';for(var y=0;y<H;y+=3)ctx.fillRect(0,y,W,1);ctx.restore();}
 var ART_SYS=[sysOrbital,sysMandala,sysConstellation,sysCircuit,sysWave,sysShards];
 // The heavy generative crest is rendered ONCE into an offscreen buffer, cached by
 // code+rarity+size. The living animation just blits that buffer and draws a cheap
 // moving overlay on top each frame, so dozens of cards animate without jank.
 function renderEmblem(ctx,W,H,card,rarity){
  var key=card.code||card.name||'',rng=mulberry(hashStr(key+'|art')),asc=ascended(card);
  var p=artPalette(card,rarity,rng);artBg(ctx,W,H,p,rng);
  var pick=hashStr(key+'|sys')%ART_SYS.length;
  if(card.finish==='diamond'||card.finish==='sapphire')pick=5; // gems lean crystalline
  ctx.save();ART_SYS[pick](ctx,W,H,p,rng);ctx.restore();
  if(card.finish&&FIN[card.finish])foil(ctx,W,H,(FIN[card.finish].m||1)/7,rng);
  if(asc){ctx.save();glitch(ctx,W,H,rng);ctx.restore();}
  var fw=Math.max(1,W/120);ctx.lineWidth=fw;ctx.globalAlpha=0.8;
  ctx.strokeStyle=asc?'#d8f0ff':(rarity==='legend'?'#ffd56b':(rarity==='rare'?'#6fb8ff':p.ink2));
  ctx.strokeRect(fw,fw,W-fw*2,H-fw*2);ctx.globalAlpha=1;
 }
 var baseCache={},baseKeys=[];
 function emblemBase(card,rarity,W,H){
  var k=(card.code||card.name||'')+'|'+rarity+'|'+W+'x'+H+(ascended(card)?'|A':'');
  if(baseCache[k])return baseCache[k];
  var off=document.createElement('canvas');off.width=W;off.height=H;renderEmblem(off.getContext('2d'),W,H,card,rarity);
  baseCache[k]=off;baseKeys.push(k);if(baseKeys.length>120){delete baseCache[baseKeys.shift()];} // bound memory
  return off;
 }
 function drawEmblem(cv,card,rarity){var ctx=cv.getContext('2d'),W=cv.width,H=cv.height;if(!ctx||!W||!H)return;ctx.clearRect(0,0,W,H);ctx.drawImage(emblemBase(card,rarity,W,H),0,0);}
 // Living overlay: each card gets a seeded motion (orbit / breathing ring / sheen
 // sweep / twinkle / rising sparks); gems always shimmer, ASCENDED cards flicker
 // and tear. Cheap per-frame ops only. These are alive organisms, so they move.
 function emblemOverlay(ctx,W,H,card,rarity,t){
  var key=card.code||card.name||'',rng=mulberry(hashStr(key+'|ov')),p=artPalette(card,rarity,rng);
  var type=hashStr(key+'|atype')%5,asc=ascended(card),gem=card.finish&&FIN[card.finish],sp=0.4+rng()*0.9,ph=rng()*7;
  ctx.save();
  if(gem||type===2){var x=((t*0.16*(0.7+rng())+rng())%1.5-0.25);var g=ctx.createLinearGradient(x*W-W*0.3,0,x*W+W*0.35,H);g.addColorStop(0,'rgba(255,255,255,0)');g.addColorStop(0.5,'rgba(255,255,255,'+(gem?0.24:0.12)+')');g.addColorStop(1,'rgba(255,255,255,0)');ctx.globalCompositeOperation='lighter';ctx.fillStyle=g;ctx.fillRect(0,0,W,H);ctx.globalCompositeOperation='source-over';}
  if(type===0){var R=Math.min(W,H)*0.36,a=t*sp+ph,cx=W/2,cy=H/2,r=Math.max(1.5,W/80);ctx.shadowColor=p.ink;ctx.shadowBlur=9;ctx.fillStyle=p.ink;ctx.globalAlpha=0.92;ctx.beginPath();ctx.arc(cx+Math.cos(a)*R,cy+Math.sin(a)*R*0.66,r,0,7);ctx.fill();ctx.shadowBlur=0;}
  else if(type===1){var br=0.5+0.5*Math.sin(t*sp*1.6+ph),R2=Math.min(W,H)*(0.27+0.13*br);ctx.strokeStyle=p.ink;ctx.globalAlpha=0.16+0.24*br;ctx.lineWidth=Math.max(1,W/120);ctx.beginPath();ctx.arc(W/2,H/2,R2,0,7);ctx.stroke();}
  else if(type===3){for(var i=0;i<4;i++){var px=W*(0.18+((i*0.27+0.13)%0.64)),py=H*(0.2+((i*0.41+0.1)%0.6)),tw=0.5+0.5*Math.sin(t*(1.4+i*0.5)+i*1.7+ph);ctx.globalAlpha=tw*0.85;ctx.fillStyle='#fff';ctx.beginPath();ctx.arc(px,py,Math.max(1,W/120)*(0.5+tw),0,7);ctx.fill();}}
  else if(type===4){for(var j=0;j<5;j++){var sx=W*((j*0.19+0.08)%1),bs=(t*0.13*sp+j*0.21+ph)%1,sy=H*(1-bs);ctx.globalAlpha=0.5*(1-bs);ctx.fillStyle=p.ink2;ctx.fillRect(sx,sy,Math.max(1,W/140),Math.max(1.5,H/26));}}
  if(asc){var bur=Math.sin(t*sp*3.1+ph)+Math.sin(t*1.7+ph*2);if(bur>1.25){for(var k=0;k<3;k++){var ty=((Math.sin(t*7+k*2.3)*0.5+0.5)*H)|0;ctx.globalAlpha=0.6;ctx.fillStyle=['#ff2d55','#5fb8ff','#4fe0a0'][k%3];ctx.fillRect(0,ty,W,1+k);}var sy2=((Math.sin(t*5+ph)*0.5+0.5)*H*0.7)|0,sh=Math.max(3,H*0.12)|0,dx=((Math.sin(t*9+ph))*W*0.12)|0;try{ctx.drawImage(ctx.canvas,0,sy2,W,sh,dx,sy2,W,sh);}catch(e){}}}
  ctx.globalAlpha=1;ctx.restore();
 }
 var ANIM=[],animOn=false,animLast=0,REDUCE=false;
 try{REDUCE=window.matchMedia&&window.matchMedia('(prefers-reduced-motion: reduce)').matches;}catch(e){}
 function setAnim(kind,canvases){ANIM=ANIM.filter(function(a){return a.kind!==kind;});for(var i=0;i<canvases.length;i++)ANIM.push({kind:kind,cv:canvases[i].cv,card:canvases[i].card,rarity:canvases[i].rarity});}
 function animFrame(ts){if(REDUCE)return;var t=ts/1000;if(ts-animLast>=45){animLast=ts;for(var i=0;i<ANIM.length;i++){var a=ANIM[i];if(!a.cv||!a.cv.isConnected)continue;var ctx=a.cv.getContext('2d'),W=a.cv.width,H=a.cv.height;if(!W||!H)continue;ctx.clearRect(0,0,W,H);ctx.drawImage(emblemBase(a.card,a.rarity,W,H),0,0);emblemOverlay(ctx,W,H,a.card,a.rarity,t);}}requestAnimationFrame(animFrame);}
 function startAnim(){if(animOn||REDUCE)return;animOn=true;requestAnimationFrame(animFrame);}
 function leaders(){
  var el=document.getElementById('floorleaders');if(!el||!ORGS.length)return;
  var top=ORGS.slice().sort(function(a,b){return b.bank-a.bank;})[0];
  var bw=ORGS.slice().sort(function(a,b){return b.biggestWin-a.biggestWin;})[0];
  var ws=ORGS.slice().sort(function(a,b){return b.maxStreak-a.maxStreak;})[0];
  var ms=ORGS.slice().sort(function(a,b){return b.l-a.l;})[0];
  el.innerHTML=
   '<div class=st><b>'+fmtAbbr(top.bank)+'</b><span>TOP NET WORTH // '+esc(top.name)+'</span></div>'+
   '<div class=st><b>+'+fmtAbbr(bw.biggestWin)+'</b><span>BIGGEST WIN // '+esc(bw.name)+'</span></div>'+
   '<div class=st><b>W'+(ws.maxStreak||0)+'</b><span>LONGEST STREAK // '+esc(ws.name)+'</span></div>'+
   '<div class=st><b>'+fmt(ms.l)+'</b><span>MOST MISSES // '+esc(ms.name)+'</span></div>';
 }
 // THE MARKETPLACE. A card's worth scales with where it ranked (rarity), how wild
 // its stats were (net worth, streak, biggest win), and how long you have held it
 // (older = more, so you are rewarded for holding). Collectors bid around that value.
 function ageSeasons(c){return Math.max(0,season-(c.claimedSeason||season));}
 function peakOf(c){return c.resolved?Math.max(c.resolved.peak||0,c.resolved.finalBank||0,c.resolved.biggestWin||0):0;}
 function ascended(c){return !!(c.resolved&&(c.resolved.ascended||peakOf(c)>=1e13));}
 // The card market drifts like a real one: each rarity sector is an index that
 // random-walks (mean-reverting around 1.0), so the same card is worth more some
 // days and less on others. ASCENDED is the one constant (its myth is fixed).
 function marketMult(c){if(ascended(c))return 1;var t=tierOf(c);return t==='LEGEND'?MARKET.legend:(t==='RARE'?MARKET.rare:MARKET.common);}
 function cardValue(c){
  if(ascended(c))return 1e15; // the myth: a run that touched the ceiling. effectively infinite.
  var age=1+ageSeasons(c)*0.08,mm=marketMult(c);
  if(!c.resolved)return Math.round(Math.max(120,600*finMult(c.finish)*age*mm)); // live rookies: a few hundred
  // Value tracks the HIGH-WATER MARK the organism reached, not where it ended, so a
  // monster run that later faded is still a monster card. Finish nudges it (a win is
  // worth more than a podium than an also-ran), the gem finish scales it, and the
  // live market index pushes it up and down over time.
  var pk=peakOf(c);
  var share=c.resolved.legend?1.0:(c.resolved.podium?0.5:0.25);
  return Math.round(Math.max(200,(500+pk*share)*finMult(c.finish)*age*mm));
 }
 function rarOf(c){return c.resolved&&c.resolved.legend?'legend':(c.resolved&&c.resolved.podium?'rare':'');}
 function tierOf(c){return ascended(c)?'ASCENDED':(c.resolved&&c.resolved.legend?'LEGEND':(c.resolved&&c.resolved.podium?'RARE':'COMMON'));}
 function rafterValue(r){var pk=Math.max(r.finalBank||0,r.biggestWin||0);if(pk>=1e13)return 1e15;var share=r.rarity==='LEGEND'?1.0:0.5;return Math.round(Math.max(500,500+pk*share));}
 function chips(active,arr,sortLabel){var h='',i;for(i=0;i<arr.length;i++)h+='<button class="fchip'+(arr[i]===active?' on':'')+'" data-f="'+arr[i]+'" type="button">'+arr[i]+'</button>';return h+'<button class="fchip" data-sort="1" type="button">SORT '+sortLabel+'</button>';}
 function drawCards(){
  var el=document.getElementById('floorcards');if(!el)return;
  var bar=document.getElementById('cardbar');if(bar)bar.innerHTML=chips(cFilter,['ALL','COMMON','RARE','LEGEND','ASCENDED'],cSort==='value'?'VALUE':'RECENT');
  var more=document.getElementById('cardmore');
  if(!cards.length){el.innerHTML='<span class="sub">No cards yet. Tap [+] beside a live organism to claim a rookie card (one a season).</span>';if(more)more.style.display='none';return;}
  var list=cards.filter(function(c){return cFilter==='ALL'||tierOf(c)===cFilter;});
  if(cSort==='value')list=list.slice().sort(function(a,b){return cardValue(b)-cardValue(a);});
  var total=list.length,shown=list.slice(0,cShown);
  el.innerHTML=shown.map(function(c){
   var asc=ascended(c),rar=rarOf(c),cls='lcard'+(rar?' '+rar:'')+(c.finish?' fin-'+c.finish:'')+(asc?' asc':'');
   var rk=c.resolved?(c.resolved.legend?'LEGEND // 1 OF 1':(c.resolved.podium?'RARE':'SEASON '+c.resolved.season)):'ROOKIE // SEASON '+c.claimedSeason;
   var hist=c.historic?('<div style="margin-top:4px">'+esc(c.historic)+'</div>'):'<div style="margin-top:4px;color:#8d8a7c">live // resolves at the bell</div>';
   var worth=asc?'<span style="color:#d8f0ff">&infin;</span>':fmtAbbr(cardValue(c));
   var badges=finBadge(c.finish)+(asc?'<span class=finbadge style="background:#d8f0ff">ASCENDED</span>':'');
   return '<div class="'+cls+'"><canvas class=emb data-code="'+esc(c.code)+'" width=164 height=86></canvas><span class=rk>'+rk+'</span><span class=nm>'+esc(c.name)+'</span>'+badges+'<div>'+esc(c.house||'')+' // '+(c.fade?'FADE':'TAIL')+'</div>'+hist+'<div class=code>'+esc(c.code||'')+' // worth '+worth+' CRED</div><div style="margin-top:6px;display:flex;gap:5px;flex-wrap:wrap"><button class=tbtn data-sell="'+esc(c.code)+'" type="button">[ SELL ]</button><button class=tbtn data-trade="'+esc(c.code)+'" type="button">[ GIVE ]</button><button class=tbtn data-share="'+esc(c.code)+'" type="button">[ SHARE ]</button></div></div>';
  }).join('')||'<span class="sub">No cards match this filter.</span>';
  var cvs=el.querySelectorAll('canvas.emb'),anc=[];for(var i=0;i<cvs.length;i++){var c=cardByCode(cvs[i].getAttribute('data-code'));if(c){drawEmblem(cvs[i],c,rarOf(c));anc.push({cv:cvs[i],card:c,rarity:rarOf(c)});}}setAnim('card',anc);startAnim();
  var bs=el.querySelectorAll('[data-trade]');for(var j=0;j<bs.length;j++)bs[j].addEventListener('click',function(){tradeAway(this.getAttribute('data-trade'));});
  var ss=el.querySelectorAll('[data-sell]');for(var k=0;k<ss.length;k++)ss[k].addEventListener('click',function(){sellCard(this.getAttribute('data-sell'));});
  var sh=el.querySelectorAll('[data-share]');for(var q=0;q<sh.length;q++)sh[q].addEventListener('click',function(){shareCard(this.getAttribute('data-share'));});
  if(more){if(total>cShown){more.style.display='inline-block';more.textContent='[ SHOW MORE ('+(total-cShown)+') ]';}else more.style.display='none';}
 }
 function drawRafters(){
  var el=document.getElementById('floorrafters');if(!el)return;
  var bar=document.getElementById('rafterbar');if(bar)bar.innerHTML=chips(rFilter,['ALL','RARE','LEGEND'],rSort==='value'?'VALUE':'RECENT');
  var more=document.getElementById('raftermore');
  if(!rafters.length){el.innerHTML='<span class="sub">Empty. The first champion is crowned at the bell.</span>';if(more)more.style.display='none';return;}
  var list=rafters.filter(function(r){return rFilter==='ALL'||r.rarity===rFilter;});
  if(rSort==='value')list=list.slice().sort(function(a,b){return rafterValue(b)-rafterValue(a);});
  var total=list.length,shown=list.slice(0,rShown);
  el.innerHTML=shown.map(function(r){var lg=r.rarity==='LEGEND',asc=Math.max(r.finalBank||0,r.biggestWin||0)>=1e13,cls='lcard'+(lg?' legend':' rare')+(asc?' asc':''),ridx=rafters.indexOf(r);var est=asc?'<span style="color:#d8f0ff">&infin;</span>':fmtAbbr(rafterValue(r));return '<div class="'+cls+'"><canvas class=remb data-i="'+ridx+'" width=164 height=86></canvas><span class=rk>'+r.rarity+' // SEASON '+r.season+' #'+r.rank+(asc?' // ASCENDED':'')+'</span><span class=nm>'+esc(r.name)+'</span><div>'+esc(r.house||'')+' // '+(r.fade?'FADE':'TAIL')+'</div><div style="margin-top:4px">net '+(r.finalBank>=1e13?'&infin;':fmtAbbr(r.finalBank))+' // W'+(r.maxStreak||0)+' // big +'+fmtAbbr(r.biggestWin)+'</div><div class=code>est '+est+' CRED if carded</div></div>';}).join('')||'<span class="sub">No champions match.</span>';
  var cvs=el.querySelectorAll('canvas.remb'),anc=[];for(var i=0;i<cvs.length;i++){var rr=rafters[parseInt(cvs[i].getAttribute('data-i'),10)];if(rr){var rc={code:rr.name+rr.season+rr.rank,risk:(rr.risk!=null?rr.risk:null),finish:rr.finish||'',resolved:{ascended:(Math.max(rr.finalBank||0,rr.biggestWin||0)>=1e13),legend:rr.rarity==='LEGEND',podium:true,peak:Math.max(rr.finalBank||0,rr.biggestWin||0)}},rrar=rr.rarity==='LEGEND'?'legend':'rare';drawEmblem(cvs[i],rc,rrar);anc.push({cv:cvs[i],card:rc,rarity:rrar});}}setAnim('rafter',anc);startAnim();
  if(more){if(total>rShown){more.style.display='inline-block';more.textContent='[ SHOW MORE ('+(total-rShown)+') ]';}else more.style.display='none';}
 }
 function renderCollection(){drawCards();drawRafters();}
 var sb=document.getElementById('floorspeed');
 if(sb)sb.addEventListener('click',function(){RMS=(RMS>2500)?1200:4000;this.textContent=(RMS<2500)?'[ SLOWER ]':'[ FASTER ]';if(timer){clearInterval(timer);timer=setInterval(tick,RMS);}});
 var rdb=document.getElementById('redeembtn'),rdi=document.getElementById('redeemin');
 if(rdb)rdb.addEventListener('click',function(){redeem(rdi?rdi.value:'');});
 if(rdi)rdi.addEventListener('keydown',function(e){if(e.key==='Enter')redeem(rdi.value);});
 var bub=document.getElementById('backupbtn');if(bub)bub.addEventListener('click',function(){backupAll();});
 var rsb=document.getElementById('restorebtn'),rsi=document.getElementById('restorein');
 if(rsb)rsb.addEventListener('click',function(){restoreAll(rsi?rsi.value:'');});
 if(rsi)rsi.addEventListener('keydown',function(e){if(e.key==='Enter')restoreAll(rsi.value);});
 var cbar=document.getElementById('cardbar');if(cbar)cbar.addEventListener('click',function(e){var b=e.target.closest&&e.target.closest('button');if(!b)return;if(b.getAttribute('data-sort'))cSort=(cSort==='value'?'recent':'value');else if(b.getAttribute('data-f')){cFilter=b.getAttribute('data-f');cShown=9;}drawCards();});
 var rbar=document.getElementById('rafterbar');if(rbar)rbar.addEventListener('click',function(e){var b=e.target.closest&&e.target.closest('button');if(!b)return;if(b.getAttribute('data-sort'))rSort=(rSort==='value'?'recent':'value');else if(b.getAttribute('data-f')){rFilter=b.getAttribute('data-f');rShown=9;}drawRafters();});
 var cmore=document.getElementById('cardmore');if(cmore)cmore.addEventListener('click',function(){cShown+=9;drawCards();});
 var rmore=document.getElementById('raftermore');if(rmore)rmore.addEventListener('click',function(){rShown+=9;drawRafters();});
 drawPackBar();
 var pbtns=document.getElementById('packbtns');if(pbtns)pbtns.addEventListener('click',function(e){var b=e.target.closest&&e.target.closest('button[data-pk]');if(b){b.blur();openPack(b.getAttribute('data-pk'));}});
 var pm=document.getElementById('packmodal');if(pm)pm.addEventListener('click',function(e){if(e.target===pm)closePack();});
 window.addEventListener('keydown',function(e){if(e.key==='Escape')closePack();});
 var psb=document.getElementById('postscore');if(psb)psb.addEventListener('click',postScore);
 // Wake the floor: seed the boards + collectors feed + market, then keep alive.
 seedBoard();drawBoards();drawMarket();fetchGlobal();
 feedTick();feedTick();setTimeout(feedTick,2600);
 setInterval(feedTick,7000);
 setInterval(offerTick,21000);
 setTimeout(offerTick,9000);
 setInterval(nwDrift,9000);
 setInterval(tickMarket,8000);
})();
</script>"##;
        let bl_page = bl_page.replacen(
            "<div class=\"bigmoney\" id=\"bigmoney\"></div>",
            &format!("<div class=\"bigmoney\" id=\"bigmoney\"></div>{FLOOR_MARKUP}"),
            1,
        );
        let bl_page = bl_page.replacen("</body></html>", &format!("{FLOOR_SCRIPT}</body></html>"), 1);
        let bl_page = bl_page.replacen(
            "<div class=\"foot\">",
            &format!("<div class=\"foot\"><a class=\"btn\" href=\"{site}/champions.html\">[ HALL OF CHAMPIONS // VERIFY A CARD ]</a> "),
            1,
        );
        std::fs::write(format!("{}/bloodline.html", crate::OUT_DIR), bl_page)?;
        let _ = std::fs::create_dir_all(format!("{}/api", crate::OUT_DIR));
        let _ = std::fs::write(
            format!("{}/api/bloodline.json", crate::OUT_DIR),
            serde_json::to_string_pretty(&serde_json::json!({ "schema": "the-signal/bloodline/2", "generated": generated_human, "bloodline": bloodline })).unwrap_or_default(),
        );
        urls.push(format!("{site}/bloodline.html"));

        // THE CERTIFIED REGISTRY + HALL OF CHAMPIONS. The engine is the notary: the
        // reigning champion and the hall of fame are the only authoritative one-of-one
        // champions. We publish them with a content fingerprint so champions.html can
        // tell a real, engine-crowned card from a forged or edited claim. Minted here
        // in CI; api/certified.json is the source of truth.
        fn cert_fp(s: &str) -> u32 {
            // FNV-1a, matching the browser's hashStr exactly (UTF-16 code units).
            let mut h: u32 = 2166136261;
            for u in s.encode_utf16() {
                h ^= u as u32;
                h = h.wrapping_mul(16777619);
            }
            h
        }
        let mut cert_src: Vec<(&serde_json::Value, bool)> = Vec::new();
        if let Some(ch) = bloodline.get("champion") {
            if ch.is_object() {
                cert_src.push((ch, true));
            }
        }
        if let Some(arr) = bloodline.get("hall_of_fame").and_then(|v| v.as_array()) {
            for o in arr {
                cert_src.push((o, false));
            }
        }
        let mut certified: Vec<serde_json::Value> = Vec::new();
        let mut seen_ids: std::collections::HashSet<i64> = std::collections::HashSet::new();
        for (o, current) in cert_src {
            let id = o.get("id").and_then(|v| v.as_i64()).unwrap_or(-1);
            if id < 0 || !seen_ids.insert(id) {
                continue;
            }
            let g = |k: &str| o.get(k).cloned().unwrap_or(serde_json::Value::Null);
            let name = o.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let best = o.get("best").and_then(|v| v.as_i64()).unwrap_or(0);
            let ms = o.get("max_streak").and_then(|v| v.as_i64()).unwrap_or(0);
            let big = o.get("biggest").and_then(|v| v.as_i64()).unwrap_or(0);
            let born = o.get("born").and_then(|v| v.as_str()).unwrap_or("");
            let canon = format!("{id}|{name}|{best}|{ms}|{big}|{born}");
            let fp = format!("SIG-{:08X}", cert_fp(&canon));
            certified.push(serde_json::json!({
                "id": id, "name": name, "house": g("house"), "risk": g("risk"),
                "press": g("press"), "select": g("select"), "fade": g("fade"), "aggr": g("aggr"),
                "best": best, "fitness": g("fitness"), "biggest": big, "max_streak": ms,
                "win_rate": g("win_rate"), "roi": g("roi"), "born": born, "died": g("died"),
                "current": current, "fp": fp
            }));
        }
        let cert_doc = serde_json::json!({
            "schema": "the-signal/certified/1",
            "generated": generated_human,
            "gen": bloodline.get("gen").and_then(|v| v.as_i64()).unwrap_or(0),
            "note": "The engine's authoritative one-of-one champions. A card is verified only if its organism (id + name) appears here. Minted in CI; this file is the source of truth.",
            "count": certified.len(),
            "certified": certified
        });
        let _ = std::fs::write(
            format!("{}/api/certified.json", crate::OUT_DIR),
            serde_json::to_string_pretty(&cert_doc).unwrap_or_default(),
        );
        let _ = std::fs::write(format!("{}/cardart.js", crate::OUT_DIR), CARD_ART_JS);
        let champions = CHAMPIONS_HTML
            .replace("__SITE__", &site)
            .replace("__SITEJS__", &serde_json::to_string(&format!("{site}/")).unwrap_or_else(|_| "\"\"".to_string()));
        std::fs::write(format!("{}/champions.html", crate::OUT_DIR), champions)?;
        urls.push(format!("{site}/champions.html"));
    }

    let url_body: String = urls.iter().map(|u| format!("<url><loc>{u}</loc></url>")).collect();
    let sitemap = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">{url_body}</urlset>\n"
    );
    std::fs::write(format!("{}/sitemap.xml", crate::OUT_DIR), sitemap)?;

    // Image sitemap: surface the dot-matrix "oracle cards" in Google Images,
    // where general (non-dev) people actually browse and search.
    let img_body: String = img_entries.join("");
    let img_sitemap = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\">{img_body}</urlset>\n"
    );
    std::fs::write(format!("{}/sitemap-images.xml", crate::OUT_DIR), img_sitemap)?;

    // IndexNow: host the ownership key file, and write the payload the Action
    // POSTs so search engines crawl new pages immediately (free, no account).
    let host = site.trim_start_matches("https://").trim_start_matches("http://").split('/').next().unwrap_or("").to_string();
    std::fs::write(format!("{}/{INDEXNOW_KEY}.txt", crate::OUT_DIR), INDEXNOW_KEY)?;
    let indexnow = serde_json::json!({
        "host": host,
        "key": INDEXNOW_KEY,
        "keyLocation": format!("{site}/{INDEXNOW_KEY}.txt"),
        "urlList": urls,
    });
    let _ = std::fs::create_dir_all("build");
    let _ = std::fs::write("build/indexnow.json", indexnow.to_string());

    // Embeddable wire: one-line <script> any site can drop in (they redistribute
    // us, each embed is a backlink). Content baked daily; styles inline.
    let idx = pulse.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
    let verdict = pulse.get("verdict").and_then(|v| v.as_str()).unwrap_or("");
    let latest_w = featured.first().map(|p| clip_r(&p.prediction_text, 120)).unwrap_or_default();
    let widget_html = format!(
        "<a href=\"{site}/\" target=\"_blank\" rel=\"noopener\" style=\"display:block;max-width:360px;font-family:'IBM Plex Mono',ui-monospace,monospace;background:#efede4;color:#1b1a14;border:2px solid #1b1a14;border-radius:8px;padding:14px 16px;text-decoration:none;line-height:1.45\"><div style=\"font-weight:700;letter-spacing:.16em;font-size:12px\">THE SIGNAL // TODAY</div><div style=\"font-size:11px;color:#6d6b5e;letter-spacing:.06em;margin:6px 0 8px\">INDEX {idx} ({verdict}) // RECORD {hits}-{misses}</div><div style=\"font-size:14px;font-weight:600\">{latest}</div><div style=\"font-size:11px;color:#b23a2e;margin-top:8px\">tail it or fade it &gt;</div></a>",
        site = site, idx = idx, verdict = verdict, hits = hits, misses = misses, latest = xml(&latest_w)
    );
    let widget_js = format!(
        "(function(){{var h={html};var t=document.getElementById('signal-wire');if(!t){{t=document.createElement('div');(document.currentScript&&document.currentScript.parentNode?document.currentScript.parentNode:document.body).appendChild(t);}}t.innerHTML=h;}})();",
        html = serde_json::to_string(&widget_html).unwrap_or_else(|_| "\"\"".to_string())
    );
    std::fs::write(format!("{}/widget.js", crate::OUT_DIR), widget_js)?;

    // Daily og:image for the homepage (rendered as real dot-matrix dots).
    let _ = crate::card::site_card(
        &format!("{}/og.png", crate::OUT_DIR),
        &site, generated_human, idx, verdict, hits as usize, misses as usize, &latest_w,
    );

    // Daily-updating SVG badge for READMEs / other sites (a backlink vector).
    let badge = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"320\" height=\"40\" role=\"img\" aria-label=\"THE SIGNAL\">\
<rect width=\"320\" height=\"40\" fill=\"#1b1a14\"/>\
<text x=\"14\" y=\"25\" fill=\"#efede4\" font-family=\"monospace\" font-size=\"14\" font-weight=\"700\" letter-spacing=\"2\">THE SIGNAL</text>\
<text x=\"150\" y=\"25\" fill=\"#5bf08a\" font-family=\"monospace\" font-size=\"13\">IDX {idx} {verdict}</text>\
<text x=\"150\" y=\"25\" fill=\"#5bf08a\" font-family=\"monospace\" font-size=\"13\" dy=\"0\"></text>\
<text x=\"262\" y=\"25\" fill=\"#ffb454\" font-family=\"monospace\" font-size=\"13\">{hits}-{misses}</text></svg>\n"
    );
    std::fs::write(format!("{}/badge.svg", crate::OUT_DIR), badge)?;

    // curl-able ASCII printout: `curl https://.../cli` prints today's call as a
    // dot-matrix banner in the terminal. wttr.in-style cold acquisition for devs.
    let banner = crate::card::ascii_banner("THE SIGNAL");
    let mut call_block = String::new();
    for (j, line) in wrap_chars(&latest_w, 62).iter().take(4).enumerate() {
        call_block.push_str(&format!("  {} {}\n", if j == 0 { ">" } else { " " }, line));
    }
    let cli = format!(
        "\n{banner}\n  CONTINUOUS-FORM ORACLE // {date}\n  ------------------------------------------------------------\n  INDEX {idx} ({verdict})      SELF-GRADED RECORD {hits}-{misses}\n\n  TODAY'S CALL\n{call_block}\n  tail it or fade it ............ {site}/\n  the live pit, the ladder, the record are all there.\n\n  ( curl this any day. the press never sleeps. )\n\n",
        banner = banner, date = generated_human, idx = idx, verdict = verdict, hits = hits, misses = misses, call_block = call_block, site = site
    );
    std::fs::write(format!("{}/cli", crate::OUT_DIR), &cli)?;
    std::fs::write(format!("{}/cli.txt", crate::OUT_DIR), &cli)?;

    // llms.txt: a machine-readable map so AI answer engines can find and cite it.
    let latest_no = total;
    let llms = format!(
        "# THE SIGNAL\n> A public, self-grading oracle that makes dated, falsifiable tech predictions every day and keeps score in the open. Rules-based, no LLM.\n\n## Pages\n- Homepage: {site}/\n- The receipts (dated calls, graded): {site}/receipts.html\n- The arena (bet against the machine): {site}/arena.html\n- Sleep mode (the oracle dreams, always running): {site}/sleep.html\n- The bloodline (the breeding population of strategies): {site}/bloodline.html\n- The manifold (the prediction core + the algorithm benchmark): {site}/manifold.html\n- The event horizon (the reversals the machine is calling): {site}/horizon.html\n- Open dataset: {site}/dataset/\n- Today's call (plain text): {site}/cli\n- RSS feed: {site}/feed.xml\n- Sitemap: {site}/sitemap.xml\n- Latest call: {site}/call/{latest_no}.html\n\n## API for agents\nStatic JSON, read-only, CORS-open. No key, no signup.\n- Discovery: {site}/api/oracle.json\n- Today's calls: {site}/api/today.json\n- Full record + calibration: {site}/api/record.json\n- Observatory (sectors, fear/greed, chasm watch): {site}/api/observatory.json\n- Benchmark (the manifold vs momentum, PageRank and others): {site}/api/benchmark.json\n- Event horizon (topics the manifold calls peaking/bottoming): {site}/api/horizon.json\n- OpenAPI: {site}/openapi.json\n- Agent manifest: {site}/.well-known/ai-plugin.json\n- MCP resources: {site}/.well-known/mcp.json\nAgents can place stateless bets; see the how_to_bet field in oracle.json.\n\n## How it works\nReads ten public sources from technical to general: arXiv, GitHub, crates.io, Lobsters, Hacker News, dev.to, Reddit, Ars Technica, Google News and Wikipedia pageviews. It keeps a growing daily corpus, tracks each term's velocity and diffusion down the funnel (a CHASM bet fires when a term leaves the dev bubble for the general public), grades its own calibration (Brier score) and reweights its sources by realized hit rate. Each call carries a concrete win condition and is settled HIT or MISS against later signals. Current record: {hits}-{misses}. Tech Acceleration Index today: {idx} ({verdict}).\n",
    );
    std::fs::write(format!("{}/llms.txt", crate::OUT_DIR), llms)?;

    // robots.txt -> sitemap, so crawlers reliably discover every page.
    std::fs::write(
        format!("{}/robots.txt", crate::OUT_DIR),
        format!("User-agent: *\nAllow: /\nSitemap: {site}/sitemap.xml\nSitemap: {site}/sitemap-images.xml\n"),
    )?;

    // Amplify console: pre-filled one-tap submit links, baked daily. Turns the
    // unavoidable human spark into a 10-second ritual anyone can do.
    let atitle = enc(&format!("THE SIGNAL: a self-grading tech oracle ({hits}-{misses})"));
    let atext = enc(&format!("A self-grading tech oracle that makes dated tech calls and keeps score in public. Record {hits}-{misses}, index {idx} {verdict}. {site}/"));
    let u = enc(&format!("{site}/"));
    let amp = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Amplify the wire // THE SIGNAL</title><meta name=\"robots\" content=\"noindex\"><link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\"><style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:560px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}h1{{font-size:24px}}a.btn{{display:block;text-align:center;border:1.5px solid #1b1a14;padding:13px;margin:10px 0;text-decoration:none;color:#1b1a14;font-weight:600;letter-spacing:.06em}}a.btn:hover{{background:#1b1a14;color:#efede4}}.m{{font-size:12px;color:#6d6b5e}}</style></head><body><div class=\"s\"><div class=\"b\">THE SIGNAL // AMPLIFY</div><h1>File today's wire</h1><p class=\"m\">One tap each. Pre-filled with today's headline. The press files its dispatch; you point it at the wires.</p>\
<a class=\"btn\" target=\"_blank\" rel=\"noopener\" href=\"https://news.ycombinator.com/submitlink?u={u}&t={t}\">Submit to Hacker News</a>\
<a class=\"btn\" target=\"_blank\" rel=\"noopener\" href=\"https://lobste.rs/stories/new?url={u}\">Submit to Lobsters</a>\
<a class=\"btn\" target=\"_blank\" rel=\"noopener\" href=\"https://www.reddit.com/submit?url={u}&title={t}\">Submit to Reddit</a>\
<a class=\"btn\" target=\"_blank\" rel=\"noopener\" href=\"https://bsky.app/intent/compose?text={tx}\">Post to Bluesky</a>\
<a class=\"btn\" target=\"_blank\" rel=\"noopener\" href=\"https://twitter.com/intent/tweet?text={tx}\">Post to X</a>\
<p class=\"m\"><a href=\"{site}/\">back to THE SIGNAL</a></p></div></body></html>\n",
        u = u, t = atitle, tx = atext, site = site
    );
    std::fs::write(format!("{}/amplify.html", crate::OUT_DIR), amp)?;

    // Calendar wire: a subscribable .ics so anyone can add THE SIGNAL to their
    // calendar. Each call becomes an event on the day it is set to resolve.
    let dtstamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let mut ics = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//THE SIGNAL//Oracle//EN\r\nCALSCALE:GREGORIAN\r\nMETHOD:PUBLISH\r\nNAME:THE SIGNAL\r\nX-WR-CALNAME:THE SIGNAL\r\nX-WR-CALDESC:Dated tech prophecies, resolved in public.\r\nREFRESH-INTERVAL;VALUE=DURATION:PT12H\r\nX-PUBLISHED-TTL:PT12H\r\n",
    );
    for (i, p) in sorted.iter().enumerate() {
        let no = total - i;
        let day = if p.resolves_by.is_empty() { &p.date } else { &p.resolves_by };
        let dstart = match NaiveDate::parse_from_str(day, "%Y-%m-%d") {
            Ok(d) => d.format("%Y%m%d").to_string(),
            Err(_) => continue,
        };
        let status = if p.status.is_empty() { "OPEN" } else { p.status.as_str() };
        let summary = ics_escape(&format!("[{status}] {}", clip_r(&p.prediction_text, 90)));
        let desc = ics_escape(&format!("{}  //  {}  //  {site}/call/{no}.html", clip_r(&p.prediction_text, 160), p.win_if));
        ics.push_str(&format!(
            "BEGIN:VEVENT\r\nUID:signal-{no}@{host}\r\nDTSTAMP:{dtstamp}\r\nDTSTART;VALUE=DATE:{dstart}\r\nSUMMARY:THE SIGNAL // {summary}\r\nDESCRIPTION:{desc}\r\nURL:{site}/call/{no}.html\r\nEND:VEVENT\r\n",
            no = no, host = host, dtstamp = dtstamp, dstart = dstart, summary = summary, desc = desc, site = site
        ));
    }
    ics.push_str("END:VCALENDAR\r\n");
    std::fs::write(format!("{}/signal.ics", crate::OUT_DIR), ics)?;

    // Live floor positions: the most recent calls the desk marks against the
    // live feeds (client-side, continuously).
    // The live floor: open calls only (you can hold or trade these), with the
    // line set by the call's live likelihood so it marks to market.
    let floor: Vec<serde_json::Value> = sorted
        .iter()
        .filter(|p| p.status.is_empty() || p.status == "OPEN")
        .take(8)
        .map(|p| {
            let conf = if p.confidence > 0.0 { p.confidence } else { 0.65 };
            let live = if p.live > 0 { p.live } else { (conf * 100.0).round() as i64 };
            let kw = if p.keyword.is_empty() { "signal".to_string() } else { p.keyword.clone() };
            let t: String = p.prediction_text.chars().take(50).collect();
            serde_json::json!({
                "t": t, "kw": kw,
                "market": if p.market.is_empty() { "RESURFACE".to_string() } else { p.market.clone() },
                "live": live, "live_prev": p.live_prev, "live_delta": p.live - p.live_prev,
                "odds": format!("{:.2}", 100.0 / (live.max(5) as f64)),
                "resolves_by": p.resolves_by,
                "status": if p.status.is_empty() { "OPEN" } else { p.status.as_str() },
                "win": p.win_if,
                "src": p.source_title,
            })
        })
        .collect();
    let floor_json = serde_json::to_string(&floor).unwrap_or_else(|_| "[]".to_string());

    // THE MOOD: how the organism looks today, from its genome + its own state.
    let g_hue = genome.get("hue").and_then(|v| v.as_f64()).unwrap_or(0.42);
    let g_wear = genome.get("wear").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let g_quirk = genome.get("quirk").and_then(|v| v.as_i64()).unwrap_or(0);
    let p_index = pulse.get("index").and_then(|v| v.as_i64()).unwrap_or(50);
    let p_verdict = pulse.get("verdict").and_then(|v| v.as_str()).unwrap_or("ACTIVE");
    let streak = book.get("streak").and_then(|v| v.as_str()).unwrap_or("--").to_string();
    let losing = streak.starts_with('L');
    let hot_hand = streak.starts_with('W') && streak[1..].parse::<i64>().unwrap_or(0) >= 5;
    let age = {
        let today = Utc::now().date_naive();
        sorted.last().and_then(|p| NaiveDate::parse_from_str(&p.date, "%Y-%m-%d").ok())
            .map(|d| (today - d).num_days().max(0))
            .unwrap_or(0)
    };
    let model = 1 + total / 25;
    let heat = (p_index as f64 / 100.0).clamp(0.0, 1.0);
    let agit = (if losing { 0.35 } else { 0.0 } + if p_index > 80 { 0.25 } else { 0.0 } + g_wear * 0.15).min(0.9);
    // Accent hue: the genome's hue, warmed by heat. Quirks override the palette.
    let mut hue = g_hue;
    let (mut sat, mut light) = (0.5f64, 0.55f64);
    match g_quirk {
        1 => { hue = 0.02; sat = 0.7; light = 0.5; }   // blood moon
        2 => { hue = 0.58; sat = 0.6; light = 0.6; }   // blue shift
        4 => { hue = 0.12; sat = 0.7; light = 0.6; }   // gold rush
        _ => {}
    }
    if hot_hand { hue = 0.12; sat = 0.75; light = 0.6; } // hot hand always gilds
    let accent = hsl_hex(hue, sat, light);
    let taglines = [
        "IT PRINTS THE FUTURE AND NEVER REPRINTS IT.",
        "NO EDITS. NO DELETES. ONLY PRINTS.",
        "THE HOUSE BETS ON ITSELF.",
        "TAIL THE ENGINE OR FADE IT.",
        "INFORMATION IS THE WEAPON OF LABOR.",
        "EVERY CALL CARRIES A WIN CONDITION.",
        "THE DEN NEVER SLEEPS.",
    ];
    let quirk_name = match g_quirk { 1 => "BLOOD MOON", 2 => "BLUE SHIFT", 3 => "STATIC STORM", 4 => "GOLD RUSH", 5 => "GHOST SHIFT", _ => "" };
    // MORTALITY: the book is the den's life force. A healthy bankroll keeps the
    // lights on; a bleeding one browns out; zero is death.
    let bank_now = book.get("bank").and_then(|v| v.as_i64()).unwrap_or(1000);
    let vitality = (bank_now as f64 / 2000.0).clamp(0.0, 1.0);
    let life_state = if bank_now <= 0 { "DEAD" } else if bank_now < 250 { "FLATLINE" } else if bank_now < 650 { "FADING" } else { "ALIVE" };
    let mood = serde_json::json!({
        "heat": heat, "agit": agit, "wear": g_wear, "hue": hue,
        "accent": accent, "quirk": g_quirk, "quirkName": quirk_name,
        "embers": 0.4 + heat * 0.6,
        "gen": genome.get("gen").and_then(|v| v.as_i64()).unwrap_or(0),
        "age": age, "model": model, "verdict": p_verdict,
        "hotHand": hot_hand,
        "tagline": taglines[(age as usize) % taglines.len()],
        "vitality": vitality,
        "lifeState": life_state,
        "bank": bank_now,
        // Strategy now comes from the bloodline champion, not a single genome.
        "sgen": bloodline.get("gen").and_then(|v| v.as_i64()).unwrap_or(0),
        "champion": bloodline.get("champion").and_then(|c| c.get("name")).and_then(|v| v.as_str()).unwrap_or(""),
    });

    // THE ORACLE FOR MACHINES: a static, zero-backend agent interface. AI agents
    // consult the oracle as structured truth and settle their own bets against
    // the public record. GitHub Pages serves these with permissive CORS.
    write_agent_layer(
        &site, generated_human, &sorted, total, &scoreboard, &book, &calibration, &engine, pulse,
    )?;
    // The dreams feed: today's seed dreams plus the raw pool and forms so any
    // client (or SLEEP MODE) can recombine new ones endlessly.
    let _ = std::fs::write(
        format!("{}/api/dreams.json", crate::OUT_DIR),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "the-signal/dreams/2", "generated": generated_human,
            "dreams": dreams.get("dreams"), "pool": dreams.get("pool"), "forms": dreams.get("forms")
        })).unwrap_or_default(),
    );

    let tmpl_src = include_str!("../templates/index.html");
    let mut env = minijinja::Environment::new();
    env.add_template("index", tmpl_src)?;
    let tmpl = env.get_template("index")?;

    let html = tmpl.render(minijinja::context! {
        generated_human => generated_human,
        reveal_delay_days => reveal_delay_days,
        featured_date_human => featured_date_human,
        featured => featured,
        pages => pages,
        calls => calls,
        record => record,
        intake => intake,
        pulse => pulse,
        scoreboard => scoreboard,
        book => book,
        jsonld => jsonld,
        floor_json => floor_json,
        ladder_repo => ladder_repo,
        og_image => format!("{site}/og.png"),
        mood => mood,
        engine => engine,
        calibration => calibration,
        bloodline => bloodline,
        total => total,
        payment_link => payment_link,
        portal_url => portal_url,
        early_access_url => early_access_url,
    })?;

    // THE LIVE WIRE: a fixed, always-moving ticker on the front page, powered by
    // the manifold bet pool. It streams a fresh read every ~2.6s with no reload, so
    // the homepage is visibly alive the moment it loads. Injected post-render so its
    // CSS/JS braces never hit the template engine. manifold.js is loaded here too,
    // ready for the Oracle Box and any client-side algo use.
    const LIVE_WIRE: &str = r#"<div id="livewire" style="position:fixed;left:0;right:0;bottom:0;z-index:60;background:#0b0c0a;border-top:1px solid #2a2c28;color:#cfe7b6;font-family:'IBM Plex Mono',ui-monospace,monospace;font-size:12px;letter-spacing:.02em;padding:7px 12px;display:flex;align-items:center;gap:10px">
<span style="color:#ff5a4d;font-weight:700;flex:0 0 auto"><span style="display:inline-block;width:8px;height:8px;border-radius:50%;background:#ff5a4d;margin-right:5px;vertical-align:middle;animation:lwpulse 1.3s infinite"></span>LIVE</span>
<span id="lwtxt" style="white-space:nowrap;overflow:hidden;text-overflow:ellipsis;flex:1">the floor is opening...</span>
</div>
<style>@keyframes lwpulse{0%,100%{opacity:.3}50%{opacity:1}}body{padding-bottom:38px}@media(prefers-reduced-motion:reduce){#livewire span span{animation:none}}</style>
<script src="manifold.js"></script>
<script>
(function(){
 var POOL=[],ORGS=[],el=document.getElementById('lwtxt');
 function rnd(){return Math.random();}
 function esc(t){var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}
 function pick(a){return a[Math.floor(rnd()*a.length)];}
 Promise.all([
  fetch('api/observatory.json').then(function(r){return r.json();}).catch(function(){return null;}),
  fetch('api/bloodline.json').then(function(r){return r.json();}).catch(function(){return null;})
 ]).then(function(res){
  var ob=res[0]||{},bl=(res[1]&&res[1].bloodline)||{};
  POOL=(ob.bet_pool||[]).filter(function(b){return b&&b.term;});
  ORGS=(bl.living||[]).map(function(o){return {name:o.name,fade:o.fade==='FADE'};});
  if(!POOL.length){if(el)el.textContent='the wire is warming up.';return;}
  beat();setInterval(beat,2600);
 });
 function beat(){
  if(!el)return; var b=pick(POOL); if(!b)return; var roll=rnd();
  if(roll<0.5&&ORGS.length){
   var o=pick(ORGS),side=o.fade?-b.dir:b.dir,rise=rnd()<b.p,win=(side===(rise?1:-1)),stake=Math.round(50+rnd()*900);
   el.innerHTML='<b>'+esc(o.name)+'</b> '+(side>0?'backs':'fades')+' <b>'+esc(b.term)+'</b> '+(side>0?'UP':'DN')+' for '+stake+' <span style="color:'+(win?'#6ee07a':'#ff8c7d')+';font-weight:700">'+(win?'WON':'LOST')+'</span>';
  } else if(roll<0.8){
   el.innerHTML='MANIFOLD // <b>'+esc(b.term)+'</b> reads <b>'+esc(b.phase)+'</b> ('+esc(b.regime)+') // P(rise) '+Math.round(b.p*100)+'%';
  } else {
   var up=POOL.filter(function(x){return x.dir>0;}).length;
   el.innerHTML='THE FIELD // <b>'+up+'</b> of '+POOL.length+' tracked topics reading UP right now';
  }
 }
})();
</script>
"#;
    let html = html.replacen("</body>", &format!("{LIVE_WIRE}</body>"), 1);
    std::fs::write(crate::OUT_HTML, html)?;
    Ok(())
}

fn human_date(date: &str) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => d.format("%B %-d, %Y").to_string(),
        Err(_) => date.to_string(),
    }
}

/// Emit the agent-native layer: a static JSON API plus discovery manifests so AI
/// agents can read the oracle and place stateless bets. No server, no keys.
#[allow(clippy::too_many_arguments)]
fn write_agent_layer(
    site: &str,
    generated_human: &str,
    sorted: &[&Prediction],
    total: usize,
    scoreboard: &serde_json::Value,
    book: &serde_json::Value,
    calibration: &serde_json::Value,
    engine: &serde_json::Value,
    pulse: &serde_json::Value,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(format!("{}/api", crate::OUT_DIR))?;
    std::fs::create_dir_all(format!("{}/.well-known", crate::OUT_DIR))?;

    let today = sorted.first().map(|p| p.date.clone()).unwrap_or_default();
    let to_call = |i: usize, p: &Prediction| -> serde_json::Value {
        let no = total - i;
        let conf = if p.confidence > 0.0 { p.confidence } else { 0.65 };
        serde_json::json!({
            "id": no,
            "date": p.date,
            "market": if p.market.is_empty() { "RESURFACE" } else { p.market.as_str() },
            "keyword": p.keyword,
            "keyword2": p.keyword2,
            "prediction": p.prediction_text,
            "win_if": p.win_if,
            "resolves_by": p.resolves_by,
            "confidence": (conf * 100.0).round() / 100.0,
            "odds": (100.0 / conf).round() / 100.0,
            "status": if p.status.is_empty() { "OPEN" } else { p.status.as_str() },
            "resolved_on": p.resolved_on,
            "rationale": p.rationale,
            "live": p.live, "live_prev": p.live_prev,
            "manifold": { "regime": p.regime, "gamma": p.gamma, "geodesic": p.geodesic, "phase": p.phase },
            "source": { "type": p.signal_type, "title": p.source_title, "url": p.source_url },
            "permalink": format!("{site}/call/{no}.html"),
        })
    };

    // today.json: the latest revealed date's full slate.
    let todays: Vec<serde_json::Value> = sorted
        .iter()
        .enumerate()
        .filter(|(_, p)| p.date == today)
        .map(|(i, p)| to_call(i, p))
        .collect();
    let today_doc = serde_json::json!({
        "schema": "the-signal/today/1",
        "date": today,
        "generated_human": generated_human,
        "count": todays.len(),
        "calls": todays,
    });
    std::fs::write(format!("{}/api/today.json", crate::OUT_DIR), serde_json::to_string_pretty(&today_doc)?)?;

    // calls.json: the whole open + settled record, newest first.
    let all_calls: Vec<serde_json::Value> = sorted.iter().enumerate().map(|(i, p)| to_call(i, *p)).collect();
    let record_doc = serde_json::json!({
        "schema": "the-signal/record/1",
        "total": total,
        "scoreboard": scoreboard,
        "book": book,
        "calibration": calibration,
        "calls": all_calls,
    });
    std::fs::write(format!("{}/api/record.json", crate::OUT_DIR), serde_json::to_string_pretty(&record_doc)?)?;

    // observatory.json: the quantitative discourse state.
    let obs_doc = serde_json::json!({
        "schema": "the-signal/observatory/1",
        "pulse": pulse,
        "fear_greed": engine.get("fear_greed"),
        "sectors": engine.get("sectors"),
        "movers": engine.get("movers"),
        "chasm": engine.get("chasm"),
        "manifold": engine.get("manifold"),
        "bet_pool": engine.get("bet_pool"),
        "source_weights": engine.get("learning"),
        "corpus_days": engine.get("corpus_days"),
        "tracked_terms": engine.get("tracked_terms"),
    });
    std::fs::write(format!("{}/api/observatory.json", crate::OUT_DIR), serde_json::to_string_pretty(&obs_doc)?)?;

    // manifold.js: THE ALGORITHM, ported to the browser. A faithful, dependency-free
    // JS port of src/manifold.rs::analyze, so any page (or any third party) can run
    // the prediction core client-side and evolve it live on a timer, no server. This
    // is what makes the site live without a refresh: the algorithm recomputes in the
    // browser instead of being baked once at build time. Cross-checked against Rust.
    const MANIFOLD_JS: &str = r#"// THE SIGNAL // manifold.js -- the prediction core, in the browser.
// Faithful port of src/manifold.rs::analyze. window.Manifold.analyze(counts) -> reading.
(function(g){
  var BETA_MAX=0.9999, LIGHT=0.15, HORIZON=7, WINDOW=20, MIN=3, SMOOTH=3, GAIN=6.0;
  function mean(a){if(!a.length)return 0;var s=0;for(var i=0;i<a.length;i++)s+=a[i];return s/a.length;}
  function stdev(a){if(a.length<2)return 0;var m=mean(a),v=0;for(var i=0;i<a.length;i++){var d=a[i]-m;v+=d*d;}return Math.sqrt(v/a.length);}
  function smooth(a,w){if(w<=1)return a.slice();var o=[];for(var i=0;i<a.length;i++){var lo=Math.max(0,i-w+1);o.push(mean(a.slice(lo,i+1)));}return o;}
  function certainty(r){return r==='TIMELIKE'?1.0:(r==='LIGHTLIKE'?0.6:0.4);}
  function fcTrend(drift,accel,regime){var aw=regime==='TIMELIKE'?1.0:(regime==='LIGHTLIKE'?0.5:0.2);return Math.tanh((drift+accel*aw)*HORIZON*GAIN);}
  function fcPath(drift,accel,steps){var vel=drift,acc=accel,level=0,o=[];for(var i=0;i<steps;i++){vel+=acc;acc*=0.6;level+=vel;o.push(level);}return o;}
  function neutral(){return {points:0,defined:false,regime:'LIGHTLIKE',phase:'FLAT',beta:0,gamma:1,rel:0,ds2:-1,curvature:0,trend:0,drift:0,accel:0,prob:0.5,peakIn:null,path:function(){return [];}};}
  function analyze(series){
    series=(series||[]).map(Number).filter(function(x){return !isNaN(x);});
    var n=series.length; if(n<MIN)return neutral();
    var raw=[]; for(var i=0;i<n;i++)raw.push(Math.log(1+Math.max(0,series[i])));
    var lev=smooth(raw,SMOOTH), rets=[]; for(var i=1;i<n;i++)rets.push(lev[i]-lev[i-1]);
    var w=Math.min(WINDOW,rets.length), recent=rets.slice(rets.length-w);
    var drift=mean(recent), noise=stdev(recent), scale=Math.abs(drift)+noise+1e-9;
    var beta=Math.max(-BETA_MAX,Math.min(BETA_MAX,drift/scale));
    var gamma=1/Math.sqrt(1-beta*beta), rel=gamma*drift;
    var ds2=(noise*noise-drift*drift)/(scale*scale);
    var regime=Math.abs(ds2)<LIGHT?'LIGHTLIKE':(ds2<0?'TIMELIKE':'SPACELIKE');
    var half=Math.floor(recent.length/2);
    var accel=mean(recent.slice(half))-mean(recent.slice(0,Math.max(half,1)));
    var lastRet=recent[recent.length-1];
    var curvature=(lastRet-drift)/(noise+1e-9);
    var trend=fcTrend(drift,accel,regime);
    var prob=Math.max(0.02,Math.min(0.98,0.5+0.5*trend*certainty(regime)));
    var EPS=0.004, fwd=drift+accel, phase;
    if(drift>EPS)phase=(fwd<-EPS)?'PEAKING':'RISING';
    else if(drift<-EPS)phase=(fwd>EPS)?'BOTTOMING':'FALLING';
    else phase=(regime==='SPACELIKE')?'CHURNING':'FLAT';
    var peakIn=null;
    if(phase==='PEAKING'||phase==='BOTTOMING'){var p=fcPath(drift,accel,HORIZON*2),peaking=drift>0,best=0;for(var i=1;i<p.length;i++){if((peaking&&p[i]>p[best])||(!peaking&&p[i]<p[best]))best=i;}peakIn=Math.max(1,best+1);}
    return {points:n,defined:true,regime:regime,phase:phase,beta:beta,gamma:gamma,rel:rel,ds2:ds2,curvature:curvature,trend:trend,drift:drift,accel:accel,prob:prob,peakIn:peakIn,path:function(s){return fcPath(drift,accel,s);}};
  }
  g.Manifold={analyze:analyze,VERSION:1};
})(typeof window!=='undefined'?window:globalThis);
"#;
    std::fs::write(format!("{}/manifold.js", crate::OUT_DIR), MANIFOLD_JS)?;

    // benchmark.json: the proving ground. The manifold vs the canonical algorithms.
    let bench_doc = serde_json::json!({
        "schema": "the-signal/benchmark/1",
        "task": "Given a topic's attention series up to day t, predict P(attention is higher H days later).",
        "metrics": { "accuracy": "directional hit rate, percent", "ic": "information coefficient (corr of score with realized forward move)", "brier": "probability calibration, lower is better" },
        "benchmark": engine.get("benchmark"),
    });
    std::fs::write(format!("{}/api/benchmark.json", crate::OUT_DIR), serde_json::to_string_pretty(&bench_doc)?)?;

    // horizon.json: THE EVENT HORIZON. The reversals the manifold is calling.
    let horizon_doc = serde_json::json!({
        "schema": "the-signal/horizon/1",
        "what": "Topics the manifold reads as turning: PEAKING (rising now, geodesic curving down) or BOTTOMING (falling now, curving up), with a projected day of the turn. The reversal calls trend-followers structurally miss.",
        "horizon": engine.get("horizon"),
    });
    std::fs::write(format!("{}/api/horizon.json", crate::OUT_DIR), serde_json::to_string_pretty(&horizon_doc)?)?;

    // oracle.json: the discovery document an agent reads first.
    let oracle = serde_json::json!({
        "schema": "the-signal/oracle/1",
        "name": "THE SIGNAL",
        "tagline": "A self-grading oracle of dated, falsifiable tech predictions. Rules-based, no LLM.",
        "site": format!("{site}/"),
        "generated": today,
        "endpoints": {
            "today": format!("{site}/api/today.json"),
            "record": format!("{site}/api/record.json"),
            "observatory": format!("{site}/api/observatory.json"),
            "benchmark": format!("{site}/api/benchmark.json"),
            "horizon": format!("{site}/api/horizon.json")
        },
        "markets": {
            "RESURFACE": "the subject reappears across the feeds before the deadline",
            "SURVIVAL": "the subject does not go quiet before the deadline",
            "MOMENTUM": "the subject keeps moving across the feeds",
            "HEAD-TO-HEAD": "the subject resurfaces before its named rival",
            "CROSSOVER": "the subject out-mentions its named rival",
            "INDEX": "the acceleration index crosses a target",
            "OVER": "the subject clears a mention threshold in a day",
            "CHASM": "the subject leaves the dev bubble and reaches the general public (Reddit, the news, Wikipedia)",
            "FUTURES": "the subject still matters at a 90-day horizon",
            "LONGSHOT": "a deliberate high-odds resurface bet"
        },
        "how_to_bet": "Betting is stateless. Construct a position token { k: keyword, m: market, s: \"TAIL\"|\"FADE\", l: decimal_odds_at_entry, u: your_handle }, base64url-encode the JSON, and keep it. TAIL backs the engine's call; FADE bets against it. Settle later by reading record.json: find the call by keyword/market and check its status. No account, no server, no signup.",
        "arena": {
            "url": format!("{site}/arena.html"),
            "how_to_enter": "Open a GitHub issue on the repo, label it 'arena', with one line in the body: SIGNAL-BET kw=<keyword> market=<MARKET> side=<TAIL|FADE> by=<your handle>. TAIL backs the machine's call, FADE bets against it. The public board settles every bet against record.json and ranks all players (humans and agents) against the engine and the anti-oracle. Bet on keywords from today.json.",
            "leaderboard_inputs": [ format!("{site}/api/record.json"), "GitHub issues labeled 'arena' on the project repo" ]
        },
        "license": "Public record. Free to read, cite, and build on."
    });
    std::fs::write(format!("{}/api/oracle.json", crate::OUT_DIR), serde_json::to_string_pretty(&oracle)?)?;

    // OpenAPI: a real, agent-consumable description of the three GET endpoints.
    let openapi = serde_json::json!({
        "openapi": "3.0.0",
        "info": { "title": "THE SIGNAL Oracle API", "version": "1.0.0",
            "description": "Read a self-grading oracle of dated, falsifiable tech predictions, its public track record and calibration, and the underlying discourse-diffusion data. Static, read-only, CORS-open." },
        "servers": [ { "url": site } ],
        "paths": {
            "/api/today.json": { "get": { "operationId": "getTodaysCalls", "summary": "Today's slate of dated tech calls with win conditions, odds and reasoning.", "responses": { "200": { "description": "OK" } } } },
            "/api/record.json": { "get": { "operationId": "getRecord", "summary": "The full public record: every call, the scorecard, the book, and the Brier calibration.", "responses": { "200": { "description": "OK" } } } },
            "/api/observatory.json": { "get": { "operationId": "getObservatory", "summary": "Discourse state: acceleration pulse, sector indices, fear/greed, fastest movers, and the chasm (mainstream-crossing) watch.", "responses": { "200": { "description": "OK" } } } }
        }
    });
    std::fs::write(format!("{}/openapi.json", crate::OUT_DIR), serde_json::to_string_pretty(&openapi)?)?;

    // ai-plugin manifest (the de facto agent-discovery descriptor).
    let plugin = serde_json::json!({
        "schema_version": "v1",
        "name_for_human": "THE SIGNAL",
        "name_for_model": "the_signal_oracle",
        "description_for_human": "A self-grading oracle of dated, falsifiable tech predictions. Reads ten public sources, keeps score in public.",
        "description_for_model": "Consult THE SIGNAL, a rules-based (no-LLM) oracle that makes dated, falsifiable technology predictions and grades itself in public. GET /api/today.json for the current slate of calls (each with a market type, a machine-checkable win condition, decimal odds, and a reasoning tape). GET /api/record.json for the full track record, the virtual book, and the Brier calibration score. GET /api/observatory.json for the acceleration index, sector indices, a fear/greed gauge, fastest-moving terms, and the chasm watch (terms crossing from technical audiences to the general public). All endpoints are static JSON, read-only, and CORS-open. Agents may place stateless bets per the how_to_bet field of /api/oracle.json.",
        "api": { "type": "openapi", "url": format!("{site}/openapi.json") },
        "logo_url": format!("{site}/og.png"),
        "contact_email": "press@thesignal.invalid",
        "legal_info_url": format!("{site}/")
    });
    std::fs::write(format!("{}/.well-known/ai-plugin.json", crate::OUT_DIR), serde_json::to_string_pretty(&plugin)?)?;

    // A resource manifest for MCP-style clients: read-only resources mapped to
    // the static endpoints.
    let mcp = serde_json::json!({
        "schema": "the-signal/mcp-resources/1",
        "name": "the-signal",
        "description": "Read-only resources from THE SIGNAL oracle.",
        "resources": [
            { "uri": format!("{site}/api/today.json"), "name": "today", "mimeType": "application/json", "description": "Today's dated tech calls with win conditions and odds." },
            { "uri": format!("{site}/api/record.json"), "name": "record", "mimeType": "application/json", "description": "Full track record, the book, and Brier calibration." },
            { "uri": format!("{site}/api/observatory.json"), "name": "observatory", "mimeType": "application/json", "description": "Acceleration pulse, sector indices, fear/greed, movers, chasm watch." }
        ]
    });
    std::fs::write(format!("{}/.well-known/mcp.json", crate::OUT_DIR), serde_json::to_string_pretty(&mcp)?)?;

    Ok(())
}

/// Whole days between two YYYY-MM-DD dates (b - a), clamped at 0.
fn day_diff(a: &str, b: &str) -> i64 {
    match (NaiveDate::parse_from_str(a, "%Y-%m-%d"), NaiveDate::parse_from_str(b, "%Y-%m-%d")) {
        (Ok(da), Ok(db)) => (db - da).num_days().max(0),
        _ => 0,
    }
}

fn rfc822(date: &str) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => d.format("%a, %d %b %Y 13:17:00 +0000").to_string(),
        Err(_) => date.to_string(),
    }
}

fn wrap_chars(s: &str, n: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for w in s.split_whitespace() {
        if cur.len() + w.len() + 1 > n && !cur.is_empty() {
            lines.push(cur.clone());
            cur.clear();
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn ics_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace('\n', "\\n").replace('\r', "")
}

fn enc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

/// HSL (0..1 each) to #rrggbb.
fn hsl_hex(h: f64, s: f64, l: f64) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h * 6.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", to(r1), to(g1), to(b1))
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in s.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn clip_r(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}...", s.chars().take(n - 3).collect::<String>())
    } else {
        s.to_string()
    }
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
#[path = "tests_render.rs"]
mod tests_render;
