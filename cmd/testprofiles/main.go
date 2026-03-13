//go:build ignore

package main

import (
	"fmt"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
)

func main() {
	profiles, err := awspkg.ListProfiles()
	fmt.Println("err:", err)
	fmt.Println("profiles:", profiles)
}
