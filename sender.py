import serial
import time
import sys

# ==========================================
# CONFIGURĂRI (Ajustează-le pentru sistemul tău)
# ==========================================
# Pe Linux este de obicei /dev/ttyACM0 sau /dev/ttyUSB0. 
# Pe Windows ar fi fost 'COM3' sau alt număr.
SERIAL_PORT = '/dev/ttyACM0' 
BAUD_RATE = 115200
GCODE_FILE = 'desen.gcode'

def send_gcode():
    try:
        # 1. Deschidem conexiunea serială cu placa STM32
        print(f"Incercam sa deschidem portul {SERIAL_PORT} la {BAUD_RATE} baud...")
        s = serial.Serial(SERIAL_PORT, BAUD_RATE, timeout=60)
        
        # Așteptăm puțin pentru ca placa să inițializeze conexiunea USB
        time.sleep(2) 
        print("Conexiune stabilita cu succes!\n")

        # 2. Deschidem fișierul care conține desenul
        with open(GCODE_FILE, 'r') as file:
            print(f"Fisierul '{GCODE_FILE}' a fost deschis. Incepem trimiterea datelor...\n")
            
            for line in file:
                # Curățăm linia de spații goale sau caractere de capăt de rând
                l = line.strip()
                
                # Ignorăm liniile goale sau comentariile din G-code (care încep cu ';')
                if not l or l.startswith(';'):
                    continue 

                # 3. Trimitem comanda către STM32 adăugând caracterul de Linie Nouă (\n)
                print(f"[PC] Trimite: {l}")
                s.write((l + '\n').encode('utf-8'))
                
                # 4. HANDSHAKING (Flow Control)
                # Aici sistemul "îngheață" și așteaptă confirmarea
                while True:
                    # Citim ce ne răspunde placa STM32
                    response = s.readline().decode('utf-8').strip()
                    
                    if response:
                        print(f"  -> [STM32] Raspuns: {response}") 
                        
                    # Dacă STM32 a procesat pașii și e gata pentru o nouă mișcare, ne trimite OK
                    if response == "OK":
                        time.sleep(0.1)
                        break 
                    
        print("\nToate comenzile au fost trimise cu succes! Desen finalizat.")
        
    except FileNotFoundError:
        print(f"Eroare: Nu am gasit fisierul '{GCODE_FILE}'. Asigura-te ca exista in folder.")
    except serial.SerialException as e:
        print(f"Eroare de conexiune seriala: {e}")
        print("Sfat: Verifica daca placa este conectata si daca portul este corect.")
        print("Pe Linux, s-ar putea sa ai nevoie de permisiuni: 'sudo chmod a+rw /dev/ttyACM0'")
    finally:
        # Ne asigurăm că eliberăm portul USB la final, indiferent dacă a dat eroare sau nu
        if 's' in locals() and s.is_open:
            s.close()
            print("Conexiunea seriala a fost inchisa in siguranta.")

if __name__ == "__main__":
    send_gcode()