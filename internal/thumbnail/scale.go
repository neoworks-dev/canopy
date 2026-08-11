package thumbnail

import (
	"image"
	"image/color"
)

// scaleToFit shrinks source so neither side exceeds size. An image already
// within the bound is returned untouched, since enlarging it would only cost
// bytes.
func scaleToFit(source image.Image, size int) image.Image {
	bounds := source.Bounds()
	width, height := fittedSize(bounds.Dx(), bounds.Dy(), size)
	if width == bounds.Dx() && height == bounds.Dy() {
		return source
	}
	return downscale(source, width, height)
}

func fittedSize(width int, height int, size int) (int, int) {
	longest := max(width, height)
	if longest <= size {
		return width, height
	}
	return atLeastOne(width * size / longest), atLeastOne(height * size / longest)
}

func atLeastOne(value int) int {
	if value < 1 {
		return 1
	}
	return value
}

// downscale averages every source pixel that falls under a destination pixel,
// which keeps thin detail from disappearing the way point sampling loses it.
func downscale(source image.Image, width int, height int) image.Image {
	destination := image.NewRGBA(image.Rect(0, 0, width, height))
	for y := range height {
		fillRow(destination, source, y)
	}
	return destination
}

func fillRow(destination *image.RGBA, source image.Image, y int) {
	bounds := destination.Bounds()
	for x := range bounds.Dx() {
		region := sourceRegion(source.Bounds(), bounds, x, y)
		destination.Set(x, y, averageOf(source, region))
	}
}

// sourceRegion is the block of source pixels one destination pixel covers,
// never empty even when the ratio rounds a side down to nothing.
func sourceRegion(source image.Rectangle, destination image.Rectangle, x int, y int) image.Rectangle {
	left := source.Min.X + x*source.Dx()/destination.Dx()
	right := source.Min.X + (x+1)*source.Dx()/destination.Dx()
	top := source.Min.Y + y*source.Dy()/destination.Dy()
	bottom := source.Min.Y + (y+1)*source.Dy()/destination.Dy()
	return image.Rect(left, top, max(right, left+1), max(bottom, top+1))
}

func averageOf(source image.Image, region image.Rectangle) color.RGBA {
	var red, green, blue, alpha uint64
	for y := region.Min.Y; y < region.Max.Y; y++ {
		red, green, blue, alpha = addRow(source, region, y, red, green, blue, alpha)
	}
	count := uint64(region.Dx() * region.Dy())
	return color.RGBA{
		R: eightBit(red / count),
		G: eightBit(green / count),
		B: eightBit(blue / count),
		A: eightBit(alpha / count),
	}
}

// Channels arrive alpha-premultiplied, so summing them needs no unpremultiply
// step to stay correct across transparent edges.
func addRow(
	source image.Image,
	region image.Rectangle,
	y int,
	red uint64, green uint64, blue uint64, alpha uint64,
) (uint64, uint64, uint64, uint64) {
	for x := region.Min.X; x < region.Max.X; x++ {
		pixelRed, pixelGreen, pixelBlue, pixelAlpha := source.At(x, y).RGBA()
		red += uint64(pixelRed)
		green += uint64(pixelGreen)
		blue += uint64(pixelBlue)
		alpha += uint64(pixelAlpha)
	}
	return red, green, blue, alpha
}

func eightBit(channel uint64) uint8 {
	return uint8(channel >> 8)
}
