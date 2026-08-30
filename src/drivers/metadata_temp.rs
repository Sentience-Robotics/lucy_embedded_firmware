pub struct DriverMetaData {
    pub base_register: u16,
    pub nb_register: u16
}

pub struct RegisterTable {
    pub registers: [u16; 32]
}

pub struct RegisterView<'a> {
    pub table: &'a mut RegisterTable,
    pub metadata: &'a DriverMetaData
}

impl<'a> RegisterView<'a> {
    pub fn new(table: &'a mut RegisterTable, metadata: &'a DriverMetaData) -> Self {
        RegisterView { table, metadata }
    }

    pub fn read_register(&self, index: u16) -> u16 {
        if index < self.metadata.nb_register {
            let reg_index = self.metadata.base_register + index;
            return self.table.registers[reg_index as usize];
        }
        0
    }

    fn write_register(&mut self, index: u16, value: u16) {
        if index < self.metadata.nb_register {
            let reg_index = self.metadata.base_register + index;
            self.table.registers[reg_index as usize] = value;
        }
    }
}