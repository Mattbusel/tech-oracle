// THE SIGNAL // manifold.js -- the prediction core, in the browser.
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
