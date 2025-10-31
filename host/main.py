import usb.core
import usb.util
import time

# Device Vendor and Product ID
VENDOR_ID = 0xc0de
PRODUCT_ID = 0xcafe

# Find the USB device
dev = usb.core.find(idVendor=VENDOR_ID, idProduct=PRODUCT_ID)

if dev is None:
    raise ValueError("Device not found. Make sure it's connected.")

# Detach kernel driver if necessary
if dev.is_kernel_driver_active(0):
    dev.detach_kernel_driver(0)

# Set configuration
dev.set_configuration()
cfg = dev.get_active_configuration()
intf = cfg[(0, 0)]

# Find the endpoints
ep_out = usb.util.find_descriptor(
    intf,
    custom_match=lambda e: usb.util.endpoint_direction(e.bEndpointAddress) == usb.util.ENDPOINT_OUT
)
ep_in = usb.util.find_descriptor(
    intf,
    custom_match=lambda e: usb.util.endpoint_direction(e.bEndpointAddress) == usb.util.ENDPOINT_IN
)

assert ep_out is not None, "OUT endpoint not found"
assert ep_in is not None, "IN endpoint not found"

print("USB device connected successfully.")

try:
    while True:
        # Send data to the device
        message = b"Hello device!"
        print(f"Sending: {message}")
        ep_out.write(message)

        # Read response (up to 64 bytes)
        try:
            response = ep_in.read(64, timeout=2000)  # 2s timeout
            print("Received:", bytes(response))
        except usb.core.USBTimeoutError:
            print("No response received (timeout)")

        time.sleep(2)

except KeyboardInterrupt:
    print("Exiting...")

finally:
    usb.util.dispose_resources(dev)
