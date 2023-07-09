g++ -c *.cpp \
	-ffreestanding        \
	-fpic              \
	-mno-red-zone		  \
	-fno-builtin -msoft-float -mno-sse -mno-sse2 -m64 -g -fno-rtti -fno-exceptions -nostdlib -nostdinc -fno-unwind-tables -fno-stack-protector -nostartfiles -nodefaultlibs

ar rcs libflyingballs.a *.o