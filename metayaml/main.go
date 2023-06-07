package main

import (
	"C"

	"github.com/google/yamlfmt"
	"github.com/google/yamlfmt/formatters/basic"
)

//export format
func format(input *C.char, output **C.char, perr **C.char) {
	ginput := C.GoString(input)

	config := basic.DefaultConfig()
	config.DropMergeTag = true
	config.Indent = 2
	config.LineLength = 100
	config.LineEnding = yamlfmt.LineBreakStyleLF
	config.RetainLineBreaks = true
	formatter := basic.BasicFormatter{
		Config:       config,
		Features:     basic.ConfigureFeaturesFromConfig(config),
		YAMLFeatures: basic.ConfigureYAMLFeaturesFromConfig(config),
	}

	out, err := formatter.Format([]byte(ginput))
	if err != nil {
		*perr = C.CString(err.Error())
		return
	}
	*output = C.CString(string(out))
}

func main() {}
