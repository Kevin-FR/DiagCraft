import AppKit

// Original DiagCraft artwork; Finder draws the actual app and Applications icons.
let args = CommandLine.arguments
if args.count != 3 { fatalError("usage: dmg_background.swift logo.png output.png") }
let width = 800, height = 480
let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height, bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false, colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
bitmap.size = NSSize(width: width, height: height)
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)!
func fill(_ color: NSColor, _ rect: NSRect) { color.setFill(); rect.fill() }
func text(_ string: String, _ x: CGFloat, _ y: CGFloat, _ size: CGFloat, _ color: NSColor, _ weight: NSFont.Weight = .regular) {
    (string as NSString).draw(at: NSPoint(x: x, y: y), withAttributes: [.font: NSFont.systemFont(ofSize: size, weight: weight), .foregroundColor: color])
}
let blue = NSColor(calibratedRed: 0.035, green: 0.25, blue: 0.63, alpha: 1)
let ink = NSColor(calibratedRed: 0.08, green: 0.13, blue: 0.22, alpha: 1)
fill(NSColor(calibratedRed: 0.97, green: 0.98, blue: 1, alpha: 1), NSRect(x: 0, y: 0, width: width, height: height))
fill(blue, NSRect(x: 0, y: 0, width: 244, height: height))
let logo = NSImage(contentsOfFile: args[1])!
logo.draw(in: NSRect(x: 42, y: 164, width: 160, height: 160), from: .zero, operation: .sourceOver, fraction: 1)
text("DIAGCRAFT", 53, 112, 19, .white, .bold)
text("Diagrammes • local • libre", 45, 87, 12, NSColor.white.withAlphaComponent(0.78))
text("Bienvenue dans DiagCraft", 290, 401, 28, ink, .bold)
text("Donnez forme à vos idées.", 291, 371, 15, ink.withAlphaComponent(0.7))
fill(NSColor(calibratedRed: 0.79, green: 0.84, blue: 0.91, alpha: 1), NSRect(x: 290, y: 348, width: 461, height: 1))
let arrow = NSBezierPath()
arrow.move(to: NSPoint(x: 505, y: 197)); arrow.line(to: NSPoint(x: 563, y: 197))
arrow.lineWidth = 3; blue.setStroke(); arrow.stroke()
let tip = NSBezierPath(); tip.move(to: NSPoint(x: 563, y: 197)); tip.line(to: NSPoint(x: 551, y: 204)); tip.line(to: NSPoint(x: 551, y: 190)); tip.close(); blue.setFill(); tip.fill()
fill(NSColor(calibratedRed: 0.79, green: 0.84, blue: 0.91, alpha: 1), NSRect(x: 290, y: 96, width: 461, height: 1))
text("GLISSEZ DIAGCRAFT VERS APPLICATIONS", 291, 70, 12, ink.withAlphaComponent(0.72), .semibold)
text("Drag DiagCraft to Applications", 291, 51, 12, ink.withAlphaComponent(0.65))
NSGraphicsContext.restoreGraphicsState()
try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: args[2]))
