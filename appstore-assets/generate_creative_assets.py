#!/usr/bin/env python3
import argparse
import math
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageEnhance, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "appstore-assets" / "creative-assets"
ICON = Image.open(ROOT / "assets" / "app-icon" / "photocomfy-mobile-1024.png").convert("RGBA")
LATIN = "/System/Library/Fonts/HelveticaNeue.ttc"
CJK = "/System/Library/Fonts/Hiragino Sans GB.ttc"


def font(size, bold=False, cjk=False):
    path = CJK if cjk else LATIN
    index = 2 if cjk and bold else 0 if cjk else 1 if bold else 0
    return ImageFont.truetype(path, max(1, round(size)), index=index)


def scaled_box(box, scale):
    return tuple(round(v * scale) for v in box)


def linear_gradient(size, stops):
    w, h = size
    positions = np.array([p for p, _ in stops], dtype=np.float32)
    colors = np.array([c for _, c in stops], dtype=np.float32)
    y = np.linspace(0, 1, h, dtype=np.float32)
    channels = [np.interp(y, positions, colors[:, i]) for i in range(3)]
    rows = np.stack(channels, axis=1).astype(np.uint8)
    return Image.fromarray(np.repeat(rows[:, None, :], w, axis=1))


def background(size, variant):
    w, h = size
    small = (max(1, w // 4), max(1, h // 4))
    sw, sh = small
    base = linear_gradient(small, [(0, (14, 15, 43)), (0.52, (27, 25, 67)), (1, (10, 22, 49))])
    arr = np.asarray(base).astype(np.float32)
    yy, xx = np.mgrid[0:sh, 0:sw].astype(np.float32)
    xx /= max(1, sw - 1)
    yy /= max(1, sh - 1)
    glows = [
        (0.12, 0.15, 0.58, (113, 103, 232), 0.48),
        (0.88, 0.72, 0.62, (47, 123, 245), 0.52),
        (0.52, 0.42, 0.38, (255, 107, 104), 0.17),
    ]
    if variant == "header":
        glows.append((0.55, 1.1, 0.65, (115, 217, 255), 0.17))
    for cx, cy, radius, color, strength in glows:
        dist = ((xx - cx) ** 2 + ((yy - cy) * sh / sw) ** 2) ** 0.5
        alpha = np.clip(1 - dist / radius, 0, 1) ** 2 * strength
        arr = arr * (1 - alpha[..., None]) + np.array(color, dtype=np.float32) * alpha[..., None]
    rng = np.random.default_rng(20261124)
    arr += rng.normal(0, 0.85, (sh, sw, 1)).astype(np.float32)
    im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8)).resize((w, h), Image.Resampling.BICUBIC).convert("RGBA")
    draw = ImageDraw.Draw(im, "RGBA")
    step = max(44, round(w / 44))
    for y in range(step, h, step):
        for x in range(step, w, step):
            if (x // step + y // step) % 3 == 0:
                r = max(1, round(w / 5200))
                draw.ellipse((x-r, y-r, x+r, y+r), fill=(193, 201, 255, 22))
    return im


def rounded_panel(im, box, radius, fill, outline=(255, 255, 255, 32), shadow=28):
    x0, y0, x1, y1 = [round(v) for v in box]
    w, h = x1 - x0, y1 - y0
    radius = max(1, round(radius))
    if shadow:
        pad = shadow * 3
        layer = Image.new("RGBA", (w + pad * 2, h + pad * 2 + shadow), (0, 0, 0, 0))
        d = ImageDraw.Draw(layer)
        d.rounded_rectangle((pad, pad, pad+w, pad+h), radius=radius, fill=(4, 6, 27, 150))
        layer = layer.filter(ImageFilter.GaussianBlur(shadow))
        im.alpha_composite(layer, (x0-pad, y0-pad+shadow))
    d = ImageDraw.Draw(im, "RGBA")
    d.rounded_rectangle((x0, y0, x1, y1), radius=radius, fill=fill, outline=outline, width=max(1, round(radius/18)))


def sparkle(draw, x, y, r, color=(255, 244, 229, 220)):
    points = [(x, y-r), (x+r*.18, y-r*.18), (x+r, y), (x+r*.18, y+r*.18),
              (x, y+r), (x-r*.18, y+r*.18), (x-r, y), (x-r*.18, y-r*.18)]
    draw.polygon(points, fill=color)


def make_scene(w, h):
    w, h = max(2, w), max(2, h)
    im = linear_gradient((w, h), [(0, (74, 66, 139)), (.34, (214, 119, 143)), (.61, (255, 190, 143)), (1, (255, 221, 174))]).convert("RGBA")
    d = ImageDraw.Draw(im, "RGBA")
    def pts(values):
        return [(round(x*w), round(y*h)) for x, y in values]
    sx, sy, sr = .73*w, .25*h, .105*h
    for mul, alpha in [(2.8, 12), (2.0, 20), (1.45, 35)]:
        r = sr*mul
        d.ellipse((sx-r, sy-r, sx+r, sy+r), fill=(255, 238, 190, alpha))
    d.ellipse((sx-sr, sy-sr, sx+sr, sy+sr), fill=(255, 243, 203, 255))
    for x, y, cw, ch, a in [(.12,.19,.28,.045,40),(.42,.12,.20,.035,28),(.60,.39,.30,.04,32)]:
        d.ellipse((x*w, y*h, (x+cw)*w, (y+ch)*h), fill=(255,228,218,a))
    d.polygon(pts([(0,.59),(.09,.43),(.17,.52),(.29,.31),(.38,.49),(.46,.39),(.58,.62),(0,.67)]), fill=(83,79,130,255))
    d.polygon(pts([(.47,.61),(.61,.45),(.69,.52),(.82,.33),(.93,.48),(1,.40),(1,.67)]), fill=(97,83,132,255))
    d.polygon(pts([(0,.68),(.10,.49),(.18,.59),(.35,.25),(.46,.53),(.54,.46),(.66,.69)]), fill=(37,45,87,255))
    d.polygon(pts([(.21,.52),(.35,.25),(.46,.53),(.39,.47),(.35,.39),(.31,.48),(.28,.44)]), fill=(208,190,205,230))
    d.polygon(pts([(.35,.25),(.46,.53),(.39,.47),(.35,.39)]), fill=(255,209,190,220))
    d.polygon(pts([(.55,.68),(.69,.48),(.76,.57),(.86,.38),(1,.58),(1,.72)]), fill=(43,48,91,255))
    water = linear_gradient((w, max(1, round(h*.43))), [(0,(70,104,151)),(.4,(38,78,126)),(1,(16,37,76))]).convert("RGBA")
    im.alpha_composite(water, (0, round(h*.57)))
    d = ImageDraw.Draw(im, "RGBA")
    for i in range(36):
        y = h*(.60 + i*.0105)
        cx = sx + math.sin(i*1.7)*w*.025
        length = w*(.018 + i*.0028)
        d.line((cx-length, y, cx+length, y), fill=(255,194,146,max(8,105-i*2)), width=max(1,round(h*.003)))
    for i in range(24):
        y = h*(.60 + i*.015)
        x = w*((i*37 % 100)/100)
        length = w*(.025 + (i%5)*.014)
        d.line((x,y,min(w,x+length),y), fill=(151,194,222,35), width=max(1,round(h*.002)))
    d.polygon(pts([(0,.69),(.10,.65),(.18,.71),(.27,.70),(.34,.78),(.26,.86),(.12,.91),(0,.88)]), fill=(20,31,60,255))
    d.polygon(pts([(0,.88),(.17,.84),(.33,.89),(.46,1),(0,1)]), fill=(12,24,48,255))
    d.polygon(pts([(1,.74),(.90,.70),(.82,.79),(.77,.90),(.72,1),(1,1)]), fill=(15,28,54,255))
    hx, hy, hw, hh = .15*w, .66*h, .055*w, .07*h
    d.rectangle((hx,hy,hx+hw,hy+hh), fill=(255,145,105,255))
    d.polygon([(hx-hw*.15,hy),(hx+hw*.48,hy-hh*.55),(hx+hw*1.15,hy)], fill=(40,37,68,255))
    d.rectangle((hx+hw*.42,hy+hh*.38,hx+hw*.67,hy+hh), fill=(45,45,74,255))
    d.rectangle((hx+hw*.12,hy+hh*.25,hx+hw*.30,hy+hh*.50), fill=(255,230,171,255))
    for bx, by, bw in [(.56,.18,.018),(.60,.15,.013),(.53,.13,.010)]:
        d.arc((bx*w,by*h,(bx+bw)*w,(by+bw*.6)*h), 190, 350, fill=(46,42,80,180), width=max(1,round(h*.003)))
    return im


def scene_card(im, box, scale=1, labels=True):
    x, y, w, h = box
    rounded_panel(im, (x,y,x+w,y+h), 42*scale, (22,25,52,255), shadow=max(2,round(30*scale)))
    d = ImageDraw.Draw(im, "RGBA")
    bar = 68*scale
    d.rounded_rectangle((x,y,x+w,y+bar), radius=38*scale, fill=(28,31,61,255))
    d.rectangle((x,y+bar*.55,x+w,y+bar), fill=(28,31,61,255))
    for i, color in enumerate([(255,120,115,220),(255,195,110,220),(113,103,232,220)]):
        cx, cy, r = x+34*scale+i*28*scale, y+bar/2, 6*scale
        d.ellipse((cx-r,cy-r,cx+r,cy+r), fill=color)
    if w > 500*scale:
        d.text((x+135*scale,y+bar/2), "COASTAL LIGHT  /  RGB", font=font(20*scale), fill=(219,222,248,165), anchor="lm")
    pad = 14*scale
    scene = make_scene(round(w-2*pad), round(h-bar-pad*1.3))
    mask = Image.new("L", scene.size, 0)
    md = ImageDraw.Draw(mask)
    md.rounded_rectangle((0,0,scene.width-1,scene.height-1), radius=25*scale, fill=255)
    scene_x, scene_y = round(x+pad), round(y+bar)
    im.paste(scene, (scene_x,scene_y), mask)
    split = x + w*.59
    original = ImageEnhance.Color(scene).enhance(.38)
    original = ImageEnhance.Contrast(original).enhance(.82)
    original = ImageEnhance.Brightness(original).enhance(.88)
    cool = Image.new("RGBA", original.size, (65,91,140,28))
    original = Image.alpha_composite(original, cool)
    left_width = max(1,round(split-scene_x))
    original_left = original.crop((0,0,left_width,original.height))
    im.paste(original_left,(scene_x,scene_y),mask.crop((0,0,left_width,mask.height)))
    top, bottom = y+bar, y+h-pad
    d.line((split,top,split,bottom), fill=(255,255,255,205), width=max(1,round(3*scale)))
    hy = top+(bottom-top)*.53
    hr = 28*scale
    d.ellipse((split-hr,hy-hr,split+hr,hy+hr), fill=(249,249,255,245), outline=(255,255,255,255), width=max(1,round(2*scale)))
    d.line((split-11*scale,hy,split+11*scale,hy), fill=(48,47,86,255), width=max(1,round(2*scale)))
    d.polygon([(split-15*scale,hy),(split-7*scale,hy-7*scale),(split-7*scale,hy+7*scale)], fill=(48,47,86,255))
    d.polygon([(split+15*scale,hy),(split+7*scale,hy-7*scale),(split+7*scale,hy+7*scale)], fill=(48,47,86,255))
    if labels and w > 600*scale:
        f = font(18*scale, bold=True)
        for label, lx in [("BEFORE",x+42*scale),("AFTER",split+28*scale)]:
            ty = top+24*scale
            bbox = d.textbbox((lx,ty),label,font=f)
            d.rounded_rectangle((bbox[0]-12*scale,bbox[1]-7*scale,bbox[2]+12*scale,bbox[3]+7*scale), radius=13*scale, fill=(12,17,44,125))
            d.text((lx,ty),label,font=f,fill=(255,255,255,225))
    d.rounded_rectangle((x,y,x+w,y+h), radius=42*scale, outline=(255,255,255,50), width=max(1,round(2*scale)))


def paste_icon(im, x, y, size):
    size = max(2, round(size))
    icon = ICON.resize((size,size), Image.Resampling.LANCZOS)
    shadow = Image.new("RGBA", (size+80,size+100), (0,0,0,0))
    alpha = Image.new("L", shadow.size, 0)
    alpha.paste(icon.getchannel("A"), (40,35))
    alpha = alpha.filter(ImageFilter.GaussianBlur(max(2,size//18)))
    shadow.putalpha(alpha.point(lambda a: a*130//255))
    im.alpha_composite(shadow, (round(x)-40,round(y)-25))
    im.alpha_composite(icon, (round(x),round(y)))


def layer_card(size, scale, label, accent):
    w, h = [round(v*scale) for v in size]
    card = Image.new("RGBA", (w,h), (0,0,0,0))
    rounded_panel(card, (0,0,w-1,h-1), 30*scale, (29,31,65,232), shadow=0)
    d = ImageDraw.Draw(card, "RGBA")
    d.text((34*scale,28*scale), "LAYERS", font=font(20*scale,bold=True), fill=(229,231,255,190))
    row_h = (h-85*scale)/3
    for i in range(3):
        yy = 72*scale+i*row_h
        d.rounded_rectangle((24*scale,yy,w-24*scale,yy+row_h-12*scale), radius=15*scale,
                            fill=(50,52,91,220) if i==0 else (40,42,77,190))
        thumb = make_scene(max(2,round(row_h*.95)),max(2,round(row_h*.68)))
        mask = Image.new("L",thumb.size,0)
        ImageDraw.Draw(mask).rounded_rectangle((0,0,thumb.width-1,thumb.height-1),radius=9*scale,fill=255)
        card.paste(thumb,(round(38*scale),round(yy+10*scale)),mask)
        d.text((58*scale+row_h,yy+row_h*.38), label if i==0 else ["Color & light","Sky mask"][i-1],
               font=font(19*scale,bold=i==0),fill=(239,239,255,230 if i==0 else 160),anchor="lm")
        d.ellipse((w-62*scale,yy+row_h*.35,w-48*scale,yy+row_h*.35+14*scale),fill=accent)
    return card


def color_panel(size, scale):
    w,h = [round(v*scale) for v in size]
    card = Image.new("RGBA",(w,h),(0,0,0,0))
    rounded_panel(card,(0,0,w-1,h-1),34*scale,(27,29,62,226),shadow=0)
    d = ImageDraw.Draw(card,"RGBA")
    d.text((34*scale,30*scale),"COLOR & CURVES",font=font(20*scale,bold=True),fill=(229,231,255,190))
    cx,cy,r = w*.28,h*.34,min(w,h)*.18
    colors = [(255,107,104,255),(255,176,114,255),(255,231,156,255),(115,217,255,255),(47,123,245,255),(189,162,255,255)]
    for i,c in enumerate(colors):
        d.pieslice((cx-r,cy-r,cx+r,cy+r),i*60-90,(i+1)*60-90,fill=c)
    d.ellipse((cx-r*.48,cy-r*.48,cx+r*.48,cy+r*.48),fill=(31,33,68,255))
    d.ellipse((cx+r*.58,cy-r*.78,cx+r*.82,cy-r*.54),fill=(255,255,255,230))
    gx0,gy0,gx1,gy1 = w*.53,h*.18,w*.90,h*.49
    d.rounded_rectangle((gx0,gy0,gx1,gy1),radius=12*scale,outline=(189,162,255,75),width=max(1,round(2*scale)))
    points=[]
    for i in range(41):
        t=i/40
        px=gx0+(gx1-gx0)*t
        py=gy1-(gy1-gy0)*(t*t*(3-2*t))
        points.append((px,py))
    d.line(points,fill=(115,217,255,230),width=max(1,round(4*scale)))
    for px,py in [points[10],points[30]]:
        rr=7*scale
        d.ellipse((px-rr,py-rr,px+rr,py+rr),fill=(255,244,229,255))
    for i,c in enumerate([(255,107,104,255),(115,217,255,255),(189,162,255,255)]):
        yy=h*(.64+i*.09)
        d.rounded_rectangle((w*.10,yy,w*.88,yy+10*scale),radius=5*scale,fill=(73,75,113,180))
        d.rounded_rectangle((w*.10,yy,w*(.56+i*.10),yy+10*scale),radius=5*scale,fill=c)
        xx=w*(.56+i*.10)
        d.ellipse((xx-9*scale,yy-4*scale,xx+9*scale,yy+14*scale),fill=(248,247,255,255))
    return card


def bezier_decoration(im, points, scale, color=(115,217,255,110)):
    p0,p1,p2,p3=points
    curve=[]
    for i in range(101):
        t=i/100
        mt=1-t
        x=mt**3*p0[0]+3*mt*mt*t*p1[0]+3*mt*t*t*p2[0]+t**3*p3[0]
        y=mt**3*p0[1]+3*mt*mt*t*p1[1]+3*mt*t*t*p2[1]+t**3*p3[1]
        curve.append((x*scale,y*scale))
    d=ImageDraw.Draw(im,"RGBA")
    d.line(curve,fill=color,width=max(1,round(4*scale)))
    for x,y in [p0,p2,p3]:
        r=10*scale
        d.ellipse((x*scale-r,y*scale-r,x*scale+r,y*scale+r),fill=(24,27,63,220),outline=color,width=max(1,round(3*scale)))


def place_rotated(im, card, xy, angle):
    rotated=card.rotate(angle,resample=Image.Resampling.BICUBIC,expand=True)
    im.alpha_composite(rotated,(round(xy[0]),round(xy[1])))


def auxiliary(im, variant, s):
    d=ImageDraw.Draw(im,"RGBA")
    w,h=im.size
    if variant=="universal":
        left=layer_card((850,500),s,"Original photo",(115,217,255,255))
        place_rotated(im,left,(280*s,990*s),7)
        right=color_panel((650,790),s)
        place_rotated(im,right,(4100*s,850*s),-6)
        bezier_decoration(im,[(150,2430),(1200,1840),(1420,2820),(2050,2380)],s)
        bezier_decoration(im,[(3420,340),(4150,40),(4470,640),(5210,390)],s,(189,162,255,100))
        sparkle(d,3940*s,480*s,48*s)
        sparkle(d,1130*s,560*s,28*s,(115,217,255,180))
    elif variant=="header":
        left=layer_card((720,400),s,"Original photo",(115,217,255,255))
        place_rotated(im,left,(100*s,660*s),7)
        right=color_panel((560,650),s)
        place_rotated(im,right,(3200*s,520*s),-6)
        bezier_decoration(im,[(50,1400),(740,1070),(900,1580),(1370,1340)],s)
        sparkle(d,3030*s,280*s,38*s)
    else:
        left=layer_card((780,460),s,"Original photo",(115,217,255,255))
        place_rotated(im,left,(40*s,1520*s),7)
        right=color_panel((590,700),s)
        place_rotated(im,right,(3260*s,1220*s),-6)
        bezier_decoration(im,[(140,600),(900,130),(1070,780),(1520,430)],s,(189,162,255,100))
        bezier_decoration(im,[(2510,2320),(3180,1860),(3480,2580),(3820,2180)],s)
        sparkle(d,3230*s,560*s,45*s)


def brand_text(im, icon_box, title_xy, title_size, tagline_xy, tagline_size, scale):
    paste_icon(im,*icon_box)
    d=ImageDraw.Draw(im,"RGBA")
    d.text((title_xy[0]*scale,title_xy[1]*scale),"PhotoComfy",font=font(title_size*scale,bold=True),fill=(250,249,255,255))
    d.text((tagline_xy[0]*scale,tagline_xy[1]*scale),"专业修图，尽在指尖",font=font(tagline_size*scale,cjk=True),fill=(218,220,247,220))


def build_universal(s=1):
    im=background((round(5244*s),round(2950*s)),"universal")
    auxiliary(im,"universal",s)
    scene_card(im,scaled_box((2040,900,1120,630),s),s)
    brand_text(im,(2010*s,690*s,190*s),(2240,720),100,(2245,835),38,s)
    d=ImageDraw.Draw(im,"RGBA")
    sparkle(d,3240*s,770*s,30*s)
    return im.convert("RGB")


def build_header(s=1):
    im=background((round(3840*s),round(1646*s)),"header")
    auxiliary(im,"header",s)
    scene_card(im,scaled_box((1320,700,1350,420),s),s,labels=False)
    brand_text(im,(1145*s,515*s,170*s),(1345,530),88,(1350,632),32,s)
    return im.convert("RGB")


def build_search(s=1):
    im=background((round(3840*s),round(2560*s)),"search")
    auxiliary(im,"search",s)
    scene_card(im,scaled_box((930,990,1370,770),s),s)
    paste_icon(im,2390*s,875*s,180*s)
    d=ImageDraw.Draw(im,"RGBA")
    d.text((2390*s,1100*s),"PhotoComfy",font=font(72*s,bold=True),fill=(250,249,255,255))
    d.text((2395*s,1235*s),"专业修图，尽在指尖",font=font(46*s,cjk=True),fill=(218,220,247,220))
    d.text((2398*s,1335*s),"图层 · 蒙版 · PSD · Apple Pencil",font=font(26*s,cjk=True),fill=(193,201,238,220))
    pill=(2395*s,1450*s,2845*s,1535*s)
    d.rounded_rectangle(pill,radius=42*s,fill=(113,103,232,55),outline=(189,162,255,90),width=max(1,round(2*s)))
    d.text((2425*s,1493*s),"本地处理  ·  无广告",font=font(29*s,cjk=True),fill=(231,226,255,240),anchor="lm")
    sparkle(d,2950*s,880*s,30*s)
    return im.convert("RGB")


def contact_sheet(images):
    sheet=Image.new("RGB",(1800,3140),(12,14,35))
    d=ImageDraw.Draw(sheet)
    d.text((80,55),"PhotoComfy · App Store Creative Assets",font=font(48,bold=True),fill=(250,249,255))
    d.text((80,125),"产品页标题 / 通用图 / 搜索结果",font=font(28,cjk=True),fill=(180,187,222))
    slots=[("universal",(80,205,1640,920),"5244 × 2950  ·  Universal / 16:9"),
           ("header",(80,1250,1640,705),"3840 × 1646  ·  Product page header / 21:9"),
           ("search",(300,2150,1200,800),"3840 × 2560 + 1920 × 1280  ·  Search / 3:2")]
    for key,(x,y,mw,mh),label in slots:
        im=images[key]
        ratio=min(mw/im.width,mh/im.height)
        thumb=im.resize((round(im.width*ratio),round(im.height*ratio)),Image.Resampling.LANCZOS)
        tx=x+(mw-thumb.width)//2
        sheet.paste(thumb,(tx,y))
        d.text((x,y+thumb.height+20),label,font=font(25),fill=(193,201,238))
    return sheet


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("--preview",action="store_true")
    args=parser.parse_args()
    if args.preview:
        target=ROOT/"target"/"tmp"/"photocomfy-creative-preview"
        target.mkdir(parents=True,exist_ok=True)
        for name,builder in [("universal",build_universal),("header",build_header),("search",build_search)]:
            im=builder(.25)
            im.save(target/f"{name}.jpg",quality=93,subsampling=0)
            print(target/f"{name}.jpg")
        return
    OUT.mkdir(parents=True,exist_ok=True)
    outputs={}
    jobs=[("universal",build_universal,"photocomfy-universal-5244x2950.png"),
          ("header",build_header,"photocomfy-title-3840x1646.png"),
          ("search",build_search,"photocomfy-search-3840x2560.png")]
    for key,builder,name in jobs:
        im=builder(1)
        path=OUT/name
        im.save(path,format="PNG",compress_level=6)
        outputs[key]=im
        print(path)
        if key=="search":
            small=im.resize((1920,1280),Image.Resampling.LANCZOS)
            small_path=OUT/"photocomfy-search-1920x1280.png"
            small.save(small_path,format="PNG",compress_level=6)
            print(small_path)
    preview=contact_sheet(outputs)
    preview_path=OUT/"photocomfy-creative-assets-preview.jpg"
    preview.save(preview_path,quality=92,subsampling=0)
    print(preview_path)


if __name__=="__main__":
    main()
